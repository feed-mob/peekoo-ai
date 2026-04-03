# AGENTS.md - peekoo-persistence-sqlite

## Overview
SQLite persistence layer with auto-discovered SQL migrations.
Migrations are embedded at compile time via `build.rs`.

## Adding a New Migration

### File naming
```
migrations/YYYYMMDDHHMM_description.sql
```
Timestamp prefix ensures lexicographic sort = chronological order.
Example: `202603261430_add_user_preferences.sql`

### Required metadata header
Each file MUST start with metadata comments before any SQL:
```sql
-- @migrate: create          # "create" or "alter"
-- @id: some_id              # optional; defaults to filename without .sql
-- @sentinel: table_name     # required for "create" — first table in SQL
-- @tolerates: "err1", "err2" # optional for "alter" — error substrings to ignore
```

### Strategy: create
For migrations that CREATE TABLE(s).
- Uses `execute_batch` with sentinel table pre-check
- MUST use `CREATE TABLE IF NOT EXISTS` in SQL
- `@sentinel` = first table created (used to detect pre-existing DB and skip SQL)
- Example:
```sql
-- @migrate: create
-- @sentinel: my_new_table

CREATE TABLE IF NOT EXISTS my_new_table (...);
```

### Strategy: alter
For migrations that ALTER existing tables.
- Splits SQL on `;`, executes each statement individually
- If `@tolerates` is set, matching errors are silently ignored
- If `@tolerates` is empty/absent, uses `execute_batch` (strict, no error tolerance)
- SQLite has no `ADD COLUMN IF NOT EXISTS`, so ALTER migrations
  that add columns MUST include `@tolerates: "duplicate column name"`
- Example:
```sql
-- @migrate: alter
-- @tolerates: "duplicate column name"

ALTER TABLE tasks ADD COLUMN new_col TEXT;
```

### SQL idempotency guidelines
- CREATE TABLE → always use `IF NOT EXISTS`
- CREATE INDEX → always use `IF NOT EXISTS`
- INSERT seed data → always use `INSERT OR IGNORE` with a WHERE guard
- ALTER TABLE ADD COLUMN → must have `@tolerates: "duplicate column name"`
- UPDATE/DELETE → only safe if WHERE clause is naturally no-op on re-run

### Discovery
No Rust code changes needed. `build.rs` scans `migrations/*.sql`,
parses metadata, and generates `MigrationDef` entries automatically.
Next `cargo build` picks up new files.

## How the runner works
`settings/store.rs` calls `run_migrations_and_seed()` which iterates
the auto-generated `MIGRATIONS` array (from `build.rs`) and applies
each migration using its declared strategy.

Tracking table: `_peekoo_migrations (id TEXT PRIMARY KEY)`
Each migration ID is recorded after successful application.

## Existing migrations
| File | @id | Strategy | Sentinel/Tolerates |
|------|-----|----------|--------------------|
| `202602150001_init.sql` | `0001_init` | create | `tasks` |
| `202602150002_agent_settings.sql` | `0002_agent_settings` | create | `agent_settings` |
| `202602150003_provider_compat.sql` | `0003_provider_compat` | create | `agent_provider_configs` |
| `202602150004_global_settings.sql` | `0004_global_settings` | create | `app_settings` |
| `202602150005_plugins.sql` | `0005_plugins` | create | `plugins` |
| `202602150006_task_extensions.sql` | `0005_task_extensions` | alter | `"duplicate column name"` |
| `202602160001_task_scheduling_and_recurrence.sql` | `0006_task_scheduling_and_recurrence` | alter | `"duplicate column name"` |
| `202602160002_recurrence_time_of_day.sql` | `0007_recurrence_time_of_day` | alter | `"duplicate column name"` |
| `202603210001_task_order_index.sql` | `0008_task_order_index` | alter | `"duplicate column name"` |
| `202603230001_agent_task_assignment.sql` | `0009_agent_task_assignment` | alter | 4 patterns (see file) |
| `202603250001_pomodoro_runtime.sql` | `0010_pomodoro_runtime` | create | `pomodoro_state` |
| `202603250002_pomodoro_autopilot.sql` | `0011_pomodoro_autopilot_v4` | alter | `"duplicate column name"` |
| `202603250003_task_finished_at.sql` | `0011_task_finished_at` | alter | `"duplicate column name"` |
| `202603250004_pomodoro_cycle_memo.sql` | `0012_pomo_memo_v1` | alter | `"duplicate column name"` |
| `202603260001_pomodoro_daily_reset.sql` | `0013_pomo_daily_reset_v1` | alter | `"duplicate column name"` |
| `202603280001_acp_runtime_v2.sql` | `0014_acp_runtime_v2` | create | `agent_sessions` |
| `202603310001_provider_config_v2.sql` | `0015_provider_config_v2` | alter | `"no such column", "duplicate column name"` |

## Testing
```bash
cargo test -p peekoo-persistence-sqlite
```
