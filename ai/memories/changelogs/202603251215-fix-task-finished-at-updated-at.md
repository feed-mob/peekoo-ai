## 2026-03-25 12:15: fix: add task finished_at and updated_at semantics

**What changed:**
- Added SQLite migration `0011_task_finished_at.sql` to introduce `tasks.finished_at` and backfill existing done rows from `updated_at`.
- Extended task DTOs and frontend task types with `updated_at` and `finished_at`.
- Updated task status transitions so entering `done` sets `finished_at`, reopening clears it, and task row mutations consistently bump `updated_at`.
- Changed Today view grouping to use `finished_at` for `Completed today` and changed Done tab sorting to prefer `finished_at` with `updated_at`/`created_at` fallbacks.
- Added frontend and Rust tests covering migration backfill and status transition timestamp behavior.

**Why:**
- `Completed today` could not be accurate for unscheduled tasks without a real completion timestamp.
- Done-tab ordering should reflect when tasks were completed, not when they were created.
- Centralized timestamp semantics reduce future task state bugs.

**Files affected:**
- `crates/persistence-sqlite/migrations/0011_task_finished_at.sql`
- `crates/persistence-sqlite/src/lib.rs`
- `crates/peekoo-agent-app/src/settings/store.rs`
- `crates/peekoo-agent-app/src/task_runtime_service.rs`
- `crates/peekoo-task-app/src/dto.rs`
- `crates/peekoo-task-app/src/sqlite_task_service.rs`
- `crates/peekoo-task-app/tests/sqlite_task_service.rs`
- `crates/peekoo-task-domain/src/task.rs`
- `apps/desktop-ui/src/types/task.ts`
- `apps/desktop-ui/src/features/tasks/utils/task-sorting.ts`
- `apps/desktop-ui/src/features/tasks/utils/task-grouping.ts`
- `apps/desktop-ui/src/features/tasks/utils/task-sorting.test.ts`
- `apps/desktop-ui/src/features/tasks/utils/task-grouping.test.ts`
- `apps/desktop-ui/src/features/tasks/utils/task-grouping-sections.test.ts`
- `apps/desktop-ui/src/features/tasks/utils/task-grouping-unscheduled-done.test.ts`
