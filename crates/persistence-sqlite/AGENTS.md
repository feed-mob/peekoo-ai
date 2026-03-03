# AGENTS.md - peekoo-persistence-sqlite

## Overview
SQLite persistence layer with embedded SQL migrations.

## Migrations
- `0001_init.sql` - Core tables: tasks, pomodoro_sessions, calendar_accounts

## Key Constants
- `MIGRATION_0001_INIT` - Embedded migration SQL

## Dependencies
None currently (raw SQL via `include_str!`)

## Testing
```bash
cargo test -p peekoo-persistence-sqlite
```

## Code Style
- Use `include_str!` for embedding SQL files
- Keep migrations in `migrations/` directory
- One migration per file with sequential numbering
