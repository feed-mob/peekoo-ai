## 2026-03-27 10:00: refactor: Migration runner table-driven loop with auto-discovery

**What changed:**
- Replaced ~300 lines of copy-pasted check-apply-record blocks with a table-driven loop
- Added `build.rs` for compile-time migration file auto-discovery
- Renamed all 15 SQL migration files to timestamp-based prefixes (e.g., `202602150001_init.sql`)
- Added metadata comment headers to each SQL file (`-- @migrate:`, `-- @id:`, `-- @sentinel:`, `-- @tolerates:`)
- Defined `MigrationDef` struct replacing 15 manual `pub const` declarations
- Extracted `apply_create_migration()` and `apply_alter_migration()` helper functions
- Added 6 validation tests for migration metadata correctness
- Updated all 4 consumer files to use `MIGRATIONS` array instead of old constants
- Updated both `AGENTS.md` files with migration creation guide

**Why:**
- The migration runner had ~370 lines of duplicated check-apply-record logic with subtle differences between blocks
- Adding new migrations required 4 manual steps (create SQL, add const, import, write inline block)
- The duplicate `0005` and `0011` prefixes showed the sequential numbering scheme was breaking down
- `build.rs` auto-discovery means adding a new migration is now just creating a single SQL file

**Files affected:**
- `crates/persistence-sqlite/build.rs` (new)
- `crates/persistence-sqlite/Cargo.toml`
- `crates/persistence-sqlite/src/lib.rs`
- `crates/persistence-sqlite/migrations/*.sql` (15 files renamed + metadata)
- `crates/peekoo-agent-app/src/settings/store.rs`
- `crates/peekoo-app-settings/src/store.rs`
- `crates/peekoo-pomodoro-app/src/lib.rs`
- `crates/peekoo-task-app/tests/sqlite_task_service.rs`
- `crates/persistence-sqlite/AGENTS.md`
- `AGENTS.md`
- `ai/memories/todo.md`
- `ai/plans/migration-runner-refactor.md` (new)
