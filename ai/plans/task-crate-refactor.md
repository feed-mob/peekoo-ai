# Plan: Task Crate Refactor

## Overview
Refactor task-related code out of `peekoo-productivity-domain` into `peekoo-task-domain` and `peekoo-task-app`, then update all consumers and remove the old crate.

## Goals
- [ ] Introduce dedicated task domain and task app crates
- [ ] Move task domain types, DTOs, service trait, and SQLite-backed implementation
- [ ] Update all workspace consumers to the new crates
- [ ] Remove `peekoo-productivity-domain`
- [ ] Keep behavior and external task APIs unchanged

## Implementation Steps

1. **Add new crates and baseline tests**
   - Create `peekoo-task-domain`
   - Create `peekoo-task-app`
   - Move or recreate the existing task unit tests first

2. **Move code into the new crates**
   - Move pure task types/events into `peekoo-task-domain`
   - Move DTOs, service trait, noop service, and SQLite task implementation into `peekoo-task-app`
   - Rename `ProductivityService` to `SqliteTaskService`

3. **Update consumers**
   - Update `peekoo-agent-app`, `peekoo-plugin-host`, `peekoo-plugin-store`, `peekoo-mcp-server`, `peekoo-agent-acp`, and `desktop-tauri`
   - Update re-exports and test imports

4. **Remove legacy crate and verify**
   - Remove `peekoo-productivity-domain` from the workspace
   - Delete the old crate directory
   - Run targeted tests/checks and fix any regressions

## Testing Strategy
- Move the existing task domain tests into `peekoo-task-domain`
- Keep task service validation tests green using `SqliteTaskService`
- Run targeted crate tests plus a workspace check for affected crates
