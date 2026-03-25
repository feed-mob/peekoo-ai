## 2026-03-25 15:30 refactor: split task code into dedicated crates

**What changed:**
- Added `crates/peekoo-task-domain` for pure task entities, enums, and task event types
- Added `crates/peekoo-task-app` for task DTOs, `TaskService`, `NoopTaskService`, and the SQLite-backed `SqliteTaskService`
- Moved the old task implementation out of `peekoo-agent-app` and updated dependent crates to use the new task crates
- Removed `crates/peekoo-productivity-domain`
- Added/retained task-focused tests in the new crates and updated task-related imports across the workspace

**Why:**
- The old `peekoo-productivity-domain` crate had become task-only and mixed pure domain types with app/service concerns
- Dedicated task crates better match the repository's domain/app architecture and the existing pomodoro split
- Renaming the concrete implementation to `SqliteTaskService` makes persistence-backed responsibilities explicit

**Files affected:**
- `Cargo.toml`
- `crates/peekoo-task-domain/**`
- `crates/peekoo-task-app/**`
- `crates/peekoo-agent-app/**`
- `crates/peekoo-plugin-host/**`
- `crates/peekoo-plugin-store/**`
- `crates/peekoo-mcp-server/**`
- `crates/peekoo-agent-acp/Cargo.toml`
- `docs/plans/2026-03-25-task-crate-refactor-design.md`
- `ai/plans/task-crate-refactor.md`
