## 2026-03-13 13:00: fix: resolve Windows "Access Denied" (OS error 5) caused by dual SQLite connections

**What changed:**
- `AgentApplication::new()` now opens a single `rusqlite::Connection` to `peekoo.sqlite`, configures it with `PRAGMA journal_mode=WAL` and `PRAGMA busy_timeout=5000`, wraps it in `Arc<Mutex<Connection>>`, and passes that shared handle to both `SettingsService` and `create_plugin_registry`.
- `SettingsStore` field changed from `Mutex<Connection>` to `Arc<Mutex<Connection>>`. Added `SettingsStore::with_conn(Arc<Mutex<Connection>>)` constructor; existing `from_path()` kept for test convenience (wraps in `Arc` internally). Extracted migration/seed logic into `run_migrations_and_seed()` helper.
- `SettingsService` gained `with_conn(Arc<Mutex<Connection>>)` and `migrate_legacy_db()` public methods. `SettingsService::new()` kept for backward-compatible test usage.
- `create_plugin_registry` now accepts `Arc<Mutex<Connection>>` instead of opening its own connection.
- Legacy DB migration now runs **before** `Connection::open()` in the production bootstrap path, because `open()` creates the file as a side-effect, which would cause the migration to exit early (it skips when the target file already exists). This was a regression caught during code review.
- Added regression test `legacy_migration_before_open_preserves_data` that proves both the correct ordering (migrate then open) and the broken ordering (open then migrate), ensuring the invariant is enforced going forward.

**Why:**
- On Windows, `AgentApplication::new()` was opening two independent `rusqlite::Connection` instances to the same `peekoo.sqlite` file: one via `SettingsService::new()` (at `settings/store.rs:38`) and another via `create_plugin_registry()` (at `application.rs:628`).
- SQLite's default `DELETE` journal mode uses mandatory file locks on Windows (unlike advisory locks on POSIX). When both connections attempted writes concurrently, the second connection hit OS error 5 ("Access Denied" / "拒绝访问").
- No `PRAGMA busy_timeout` was set, so contention failed immediately instead of retrying.
- WAL mode allows concurrent readers and serialises writers gracefully. Combined with `busy_timeout=5000`, even future additional connections would tolerate brief contention instead of failing.

**Critical ordering invariant:**
- `SettingsService::migrate_legacy_db(&db_path)` **must** be called before `Connection::open(&db_path)` in the bootstrap path. `Connection::open` creates the file, and the migration exits early if the target file exists. Reversing the order silently drops legacy data on upgrade. This is enforced by the `legacy_migration_before_open_preserves_data` test.

**Architecture note:**
- `peekoo-plugin-host` (`PluginRegistry`, `PermissionStore`, `PluginStateStore`) already accepted `Arc<Mutex<Connection>>` -- no changes needed in that crate.
- The shared connection carries all migrations (settings + plugin tables) since they live in the same `peekoo.sqlite` database via `0001_init.sql`, `0002_agent_settings.sql`, and `0003_provider_compat.sql`.
- The `from_path()` / `new()` constructors remain available for test isolation where each test opens its own temporary database.

**Files affected:**
- `crates/peekoo-agent-app/src/application.rs` (single connection + pragmas, migration before open, pass to both subsystems)
- `crates/peekoo-agent-app/src/settings/store.rs` (`conn` field -> `Arc<Mutex<Connection>>`, `with_conn` constructor, `run_migrations_and_seed` helper)
- `crates/peekoo-agent-app/src/settings/mod.rs` (`migrate_legacy_db` + `with_conn` on `SettingsService`, regression test)

**Verification:**
- `cargo check` -- clean, 0 errors
- `cargo clippy` -- clean, 0 warnings
- `cargo test` -- all 137 tests pass across all crates (1 new regression test)
