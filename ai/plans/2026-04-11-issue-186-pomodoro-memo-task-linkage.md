# Plan: Issue 186 Pomodoro Memo Task Linkage

## Overview

Add task selection to the post-focus pomodoro memo flow so a completed focus session can be linked to a task and displayed consistently in pomodoro history.

## Goals

- [x] Add task selection to the post-focus memo window
- [x] Persist the selected task on the pomodoro cycle history record
- [x] Snapshot the selected task title for stable history display
- [x] Show the linked task in pomodoro history
- [x] Cover backend and frontend behavior with tests

## Design

### Approach

- Extend the built-in pomodoro cycle history model with optional task linkage fields instead of encoding task data into memo text.
- Keep the post-focus flow centered in `PomodoroMemoView`, which already owns memo submission and is the narrowest place to add task selection.
- Reuse the existing task list command and select primitive instead of introducing a new task picker abstraction.
- Keep memo save focused on pomodoro history linkage; do not mutate task activity from this flow.

### Components

- `crates/persistence-sqlite/migrations/*`: add `task_id` and `task_title` columns to `pomodoro_cycle_history`
- `crates/peekoo-pomodoro-app`: persist and return pomodoro task linkage fields
- `crates/peekoo-agent-app`: extend pomodoro memo save API shape
- `apps/desktop-tauri/src-tauri`: pass the optional task id through the Tauri command
- `apps/desktop-ui/src/features/pomodoro/tool-client.ts`: send the optional task id from the UI
- `apps/desktop-ui/src/views/PomodoroMemoView.tsx`: load tasks, allow selection, and save memo with task linkage
- `apps/desktop-ui/src/features/pomodoro/PomodoroPanel.tsx`: display the linked task in history

## Implementation Steps

1. **Add backend tests first**
   - Add failing pomodoro app tests for saving memo with `task_id` and `task_title`
   - Verify history returns the linked task fields

2. **Add persistence and DTO support**
   - Add a migration for `pomodoro_cycle_history.task_id` and `pomodoro_cycle_history.task_title`
   - Extend pomodoro DTOs and history queries
   - Extend memo save plumbing from UI command to app service

3. **Add frontend tests first**
   - Add failing tests for memo submission including selected task id
   - Add coverage for task linkage payload passthrough

4. **Implement the memo task selection flow**
   - Load tasks in the memo window
   - Add a compact select control for choosing an active task
   - Save memo with optional task id
   - Persist selected task linkage when saving memo

5. **Expose the linkage in history and verify**
   - Show linked task title in pomodoro history details
   - Run targeted Rust tests and frontend tests
   - Run a frontend build or type check to catch integration regressions

## Files to Modify/Create

- `ai/plans/2026-04-11-issue-186-pomodoro-memo-task-linkage.md`
- `crates/persistence-sqlite/migrations/*` - new pomodoro cycle linkage migration
- `crates/peekoo-pomodoro-app/src/lib.rs` - DTOs, save logic, history queries, tests
- `crates/peekoo-agent-app/src/application.rs` - pomodoro memo save API extension
- `apps/desktop-tauri/src-tauri/src/lib.rs` - Tauri command argument extension
- `apps/desktop-ui/src/features/pomodoro/tool-client.ts` - typed client update
- `apps/desktop-ui/src/features/pomodoro/tool-client.test.ts` - client tests
- `apps/desktop-ui/src/views/PomodoroMemoView.tsx` - task selection and submit flow
- `apps/desktop-ui/src/features/pomodoro/PomodoroPanel.tsx` - linked task display in history

## Testing Strategy

- `cargo test -p peekoo-pomodoro-app`
- Targeted Rust tests covering memo save and history projection
- Frontend tests for the typed pomodoro client and memo submission flow
- `bun run build` in `apps/desktop-ui`

## Status

- Completed end-to-end implementation for issue `#186`
- Added pomodoro cycle task-link persistence and history projection
- Added post-focus task selection UI with pomodoro history task linkage
- Verified with targeted Rust tests, crate checks, frontend tests, and desktop UI production build

## Open Questions

- None. The selected task should be persisted on the pomodoro cycle and displayed in history.
