## 2026-03-09 16:00: feat: Unified logging system with file persistence

**What changed:**
- Added `tauri-plugin-log` (v2, `tracing` feature) and `log = "0.4"` to `desktop-tauri`
- Initialized the log plugin in `run()` with:
  - Dev builds (`cfg!(debug_assertions)`): logs write to `<project-root>/logs/` via `PEEKOO_PROJECT_ROOT` env var
  - Release builds: logs write to platform `LogDir` (`~/.local/share/com.peekoo.desktop/logs/` on Linux)
  - Size-based rotation: 5 MB max, keep last 5 files
  - Level controlled by `RUST_LOG` env var; defaults to `error` in production
- Added `log:default` capability permission so the frontend can invoke the log plugin
- Added `@tauri-apps/plugin-log` to `desktop-ui`
- Added `src/lib/log.ts`: `forwardConsole()` rewrites `console.*` to forward messages through the Tauri log bridge, with `%s`/`%d`/`%o` format string expansion and object JSON-serialisation
- `main.tsx` calls `forwardConsole()` at startup; all existing `console.error/warn` calls now persist to file
- Added `peekoo_log_dir()` to `peekoo-paths` crate (returns `peekoo_global_data_dir()/logs`) with a unit test
- Updated `justfile` `dev` recipe: sets `RUST_LOG=trace` and `PEEKOO_PROJECT_ROOT="$(pwd)"` so dev logs land at the repo root
- Added `/logs/` to `.gitignore`
- Existing `tracing::*` calls in `peekoo-plugin-host` and `peekoo-agent-app` now captured automatically via the `tracing` bridge feature

**Why:**
- No persistent logging existed; all `tracing::*` calls were silently discarded (no subscriber)
- `eprintln!` is invisible in release builds on Windows
- Errors were hard to debug without a log trail

**Files affected:**
- `apps/desktop-tauri/src-tauri/Cargo.toml` — new deps
- `apps/desktop-tauri/src-tauri/src/lib.rs` — log plugin init, conditional file target
- `apps/desktop-tauri/src-tauri/capabilities/default.json` — `log:default` permission
- `apps/desktop-ui/package.json` — `@tauri-apps/plugin-log`
- `apps/desktop-ui/src/lib/log.ts` — new: console forwarding utility
- `apps/desktop-ui/src/main.tsx` — calls `forwardConsole()`
- `crates/peekoo-paths/src/lib.rs` — new `peekoo_log_dir()` + test
- `justfile` — `RUST_LOG=trace` + `PEEKOO_PROJECT_ROOT` in dev recipe
- `.gitignore` — `/logs/`
- `docs/plans/2026-03-09-logging-system-design.md` — design document
