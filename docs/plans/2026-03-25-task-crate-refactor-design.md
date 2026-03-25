# Task Crate Refactor Design

## Overview

Refactor task-related code out of `peekoo-productivity-domain` into dedicated task crates that match the current architecture direction:

- `crates/peekoo-task-domain` for pure task domain concepts
- `crates/peekoo-task-app` for task application contracts and SQLite-backed orchestration

This change should fully replace `peekoo-productivity-domain` rather than keeping a compatibility shim.

## Goals

- Separate pure task domain logic from application/service concerns
- Align task architecture with the existing dedicated pomodoro crates
- Remove the misleading `peekoo-productivity-domain` crate
- Preserve existing task DTOs, service signatures, and external behavior
- Avoid database schema changes and transport API changes during the refactor

## Chosen Approach

Use a full split now with direct consumer updates.

### Why this approach

- It creates clean boundaries immediately
- It avoids a second migration later
- It matches the user's desired end state
- The current `peekoo-productivity-domain` is effectively task-only, so keeping it would add confusion rather than value

## Alternatives Considered

### 1. Full split now with direct updates — chosen
Create both new crates, update all consumers, and remove `peekoo-productivity-domain` in the same change.

**Pros**
- Clean boundaries immediately
- No long-lived compatibility layer
- Best architectural clarity

**Cons**
- Touches multiple crates in one refactor

### 2. Split now with temporary compatibility shim
Keep `peekoo-productivity-domain` briefly as a re-export crate.

**Pros**
- Lower short-term migration risk

**Cons**
- Leaves misleading structure in place
- Adds follow-up cleanup work

### 3. Incremental migration over several changes
Create the new crates and move consumers gradually.

**Pros**
- Smaller changes per step

**Cons**
- Longer-lived duplication
- More total churn

## Target Architecture

### `crates/peekoo-task-domain`
Owns pure task concepts only.

**Contents**
- `Task`
- `TaskStatus`
- `TaskPriority`
- `AgentWorkStatus`
- `TaskEvent`
- `TaskEventType`

**Rules**
- No persistence concerns
- No transport or UI concerns
- No Tauri dependencies
- No SQLite dependencies
- Keep domain constructors and invariant-preserving methods here

### `crates/peekoo-task-app`
Owns task application contracts and orchestration.

**Contents**
- `TaskDto`
- `TaskEventDto`
- `TaskService`
- `NoopTaskService`
- SQLite-backed task service implementation moved from `peekoo-agent-app`
- Mapping and parsing helpers between DB values, domain enums, and DTO fields

**Rules**
- May depend on `peekoo-task-domain`
- May depend on `rusqlite`, `uuid`, `chrono`, `serde_json`, and similar support crates
- Must not depend on Tauri or frontend crates

### Removal

Remove `crates/peekoo-productivity-domain` from the workspace after consumers are updated.

## Proposed Module Layout

### `peekoo-task-domain`

Initial layout:

- `crates/peekoo-task-domain/src/lib.rs`
- `crates/peekoo-task-domain/src/task.rs`

Optional future split if needed:

- `crates/peekoo-task-domain/src/task_event.rs`

Start with a single `task.rs` to keep the migration focused.

### `peekoo-task-app`

Proposed layout:

- `crates/peekoo-task-app/src/lib.rs`
- `crates/peekoo-task-app/src/dto.rs`
- `crates/peekoo-task-app/src/service.rs`
- `crates/peekoo-task-app/src/sqlite_task_service.rs`
- optional `crates/peekoo-task-app/src/mappers.rs`

## Naming Decisions

Rename the current concrete task implementation from `ProductivityService` to `SqliteTaskService`.

### Why `SqliteTaskService`

- It is precise about responsibility
- It reflects that the implementation is persistence-backed
- It avoids vague naming like `ProductivityService` once tasks are isolated

## Current Usage Impact

Current task API usage exists across multiple crates, including:

- `crates/peekoo-agent-app`
- `crates/peekoo-plugin-host`
- `crates/peekoo-plugin-store`
- `crates/peekoo-mcp-server`
- `crates/peekoo-agent-acp`
- `apps/desktop-tauri`

These consumers should be updated to import from the new task crates directly.

## Dependency Direction

Desired dependency flow after refactor:

- transport, plugin, and app integration layers depend on `peekoo-task-app`
- `peekoo-task-app` depends on `peekoo-task-domain`
- pure domain consumers depend on `peekoo-task-domain` only

This keeps domain concerns below application orchestration and transport concerns.

## Data and API Compatibility

This refactor should preserve external behavior.

### Preserve exactly

- `TaskDto` fields and serialized names
- `TaskEventDto` fields and serialized names
- `TaskService` method signatures
- status strings such as `todo`, `in_progress`, `done`, `cancelled`
- event type string encodings
- Tauri command payload and response shapes
- existing SQLite schema

### Avoid in this change

- Database migrations
- Frontend contract changes
- New task features
- Renaming DTO fields
- Semantic changes to validation or event creation behavior

## Risks and Mitigations

### Risk: import churn across many crates
A large number of files currently import from `peekoo_productivity_domain::task`.

**Mitigation**
- Keep module names predictable
- Update imports systematically
- Run workspace-wide search to verify no references remain

### Risk: mixing persistence details into the app contract surface
The concrete task implementation is SQLite-backed.

**Mitigation**
- Keep the implementation isolated in `sqlite_task_service.rs`
- Expose trait and DTOs separately from the concrete implementation
- Use the explicit `SqliteTaskService` name

### Risk: behavior drift during the move
Helpers for parsing status, serializing priority, and recording events are easy to alter accidentally.

**Mitigation**
- Preserve signatures and helper behavior
- Add regression tests before moving code where coverage is missing
- Keep the move mostly structural

### Risk: confusing re-exports in `peekoo-agent-app`
`peekoo-agent-app` currently re-exports task types from the old crate.

**Mitigation**
- Update re-exports to use `peekoo-task-app`
- Remove naming that still implies productivity ownership

## Testing Strategy

This is a refactor, so the priority is proving behavior remains unchanged.

### `peekoo-task-domain`
Move and keep existing unit tests covering:

- default task status
- default assignee
- status transitions
- label deduplication
- label removal

### `peekoo-task-app`
Add or preserve focused coverage for:

- task creation validation
- status parsing and serialization
- task toggle and update behavior
- task event creation behavior
- loading and listing task DTOs from storage

### Integration confidence
Run existing checks and tests that exercise task flows, especially in:

- `peekoo-agent-app`
- task-related plugin host and plugin store tests
- workspace compilation paths that import task DTOs and services

## Implementation Outline

1. Add `crates/peekoo-task-domain`
2. Add `crates/peekoo-task-app`
3. Move pure task types and tests into `peekoo-task-domain`
4. Move DTOs, `TaskService`, `NoopTaskService`, and the SQLite-backed implementation into `peekoo-task-app`
5. Rename `ProductivityService` to `SqliteTaskService`
6. Update imports and re-exports across dependent crates
7. Remove `peekoo-productivity-domain` from the workspace
8. Run checks and tests to confirm no behavior changes

## Success Criteria

- `peekoo-productivity-domain` is removed
- no references to `peekoo_productivity_domain` remain in active code
- task domain types compile from `peekoo-task-domain`
- task DTOs and service contracts compile from `peekoo-task-app`
- external behavior remains unchanged
- workspace tests and checks pass
- architecture clearly separates task domain from task application concerns
