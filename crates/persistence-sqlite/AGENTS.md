# AGENTS.md - peekoo-persistence-sqlite

## Overview
SQLite persistence layer with embedded SQL migrations.

## Migrations
- `0001_init.sql` - Baseline tables for tasks, pomodoro sessions, conversations/messages, and legacy calendar/plugin/event data
- `0002_agent_settings.sql` - Agent settings, provider auth, and skill tables
- `0003_provider_compat.sql` - Provider compatibility/config tables

## Key Constants
- `MIGRATION_0001_INIT` - Embedded migration SQL
- `MIGRATION_0002_AGENT_SETTINGS` - Embedded agent settings migration
- `MIGRATION_0003_PROVIDER_COMPAT` - Embedded provider config migration

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
