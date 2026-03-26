# Plan: Refactor Migration Runner into Table-Driven Loop with Auto-Discovery

## Overview
Replace ~300 lines of copy-pasted check-apply-record blocks in `settings/store.rs` with a table-driven loop, auto-discovery via `build.rs`, and metadata-declared strategies in SQL files.

## Why not rusqlite_migration / refinary
- Both crates stop on first error
- SQLite has no `ADD COLUMN IF NOT EXISTS`
- Our ALTER migrations need error tolerance ("duplicate column name" filtering)
- No popular crate provides this out of the box

## Approach: Option C — Custom with build.rs (zero new deps)

### 1. Rename all SQL files to timestamp-based names

| Old filename | New filename | @id (backward compat) |
|---|---|---|
| 0001_init.sql | 202602150001_init.sql | 0001_init |
| 0002_agent_settings.sql | 202602150002_agent_settings.sql | 0002_agent_settings |
| 0003_provider_compat.sql | 202602150003_provider_compat.sql | 0003_provider_compat |
| 0004_global_settings.sql | 202602150004_global_settings.sql | 0004_global_settings |
| 0005_plugins.sql | 202602150005_plugins.sql | 0005_plugins |
| 0005_task_extensions.sql | 202602150006_task_extensions.sql | 0005_task_extensions |
| 0006_task_scheduling_and_recurrence.sql | 202602160001_task_scheduling_and_recurrence.sql | 0006_task_scheduling_and_recurrence |
| 0007_recurrence_time_of_day.sql | 202602160002_recurrence_time_of_day.sql | 0007_recurrence_time_of_day |
| 0008_task_order_index.sql | 202603210001_task_order_index.sql | 0008_task_order_index |
| 0009_agent_task_assignment.sql | 202603230001_agent_task_assignment.sql | 0009_agent_task_assignment |
| 0010_pomodoro_runtime.sql | 202603250001_pomodoro_runtime.sql | 0010_pomodoro_runtime |
| 0011_pomodoro_autopilot.sql | 202603250002_pomodoro_autopilot.sql | 0011_pomodoro_autopilot_v4 |
| 0011_task_finished_at.sql | 202603250003_task_finished_at.sql | 0011_task_finished_at |
| 0012_pomodoro_cycle_memo.sql | 202603250004_pomodoro_cycle_memo.sql | 0012_pomo_memo_v1 |
| 0013_pomodoro_daily_reset.sql | 202603260001_pomodoro_daily_reset.sql | 0013_pomo_daily_reset_v1 |

### 2. Add metadata header to each SQL file

Format:
```sql
-- @migrate: create | alter
-- @id: <migration_id>              (optional; defaults to filename without .sql)
-- @sentinel: <table_name>          (required for create)
-- @tolerates: "err1", "err2"       (optional for alter)
```

### 3. Create build.rs

- Scans `migrations/*.sql` at compile time
- Parses metadata from header comments
- Generates `$OUT_DIR/migrations.rs` with static `MIGRATIONS: &[MigrationDef]`
- Emits `cargo:rerun-if-changed=migrations`

### 4. Update lib.rs

- Define `MigrationDef` struct
- `include!` generated migrations
- New tests: metadata validation, sort order, create-table presence

### 5. Refactor store.rs

- Replace 300-line inline code with loop over `MIGRATIONS`
- Extract: `apply_create_migration()`, `apply_alter_migration()`
- Extract: `is_migration_applied()`, `record_migration()`, `sqlite_table_exists()`
- Remove old 15 `MIGRATION_*` imports and `apply_migration_if_needed()`

### 6. New migration convention

New files: `YYYYMMDDHHMM_description.sql`
- No Rust code changes needed
- `build.rs` auto-discovers on next build
- `@id` optional (defaults to filename)

## Files modified
- `crates/persistence-sqlite/migrations/*.sql` — rename + add metadata
- `crates/persistence-sqlite/build.rs` — new
- `crates/persistence-sqlite/Cargo.toml` — add `build = "build.rs"`
- `crates/persistence-sqlite/src/lib.rs` — replace constants with MigrationDef
- `crates/peekoo-agent-app/src/settings/store.rs` — refactor runner

## Verification
- `cargo test -p peekoo-persistence-sqlite`
- `cargo test -p peekoo-agent-app`
- `just check`
- `just lint`
