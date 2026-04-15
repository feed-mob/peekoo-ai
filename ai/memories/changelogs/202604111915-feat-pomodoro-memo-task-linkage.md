# feat: Pomodoro memo task linkage

## Date
2026-04-11

## Summary
Linked the post-focus pomodoro memo flow to tasks so users can choose a task when saving a focus memo and persist that task on pomodoro history.

## Changes

### Modified: `crates/persistence-sqlite/`
- Added `202604110001_pomodoro_cycle_task_link.sql` to extend `pomodoro_cycle_history` with `task_id` and `task_title`

### Modified: `crates/peekoo-pomodoro-app/`
- Extended `PomodoroCycleDto` with linked-task fields
- Updated memo save logic and history queries to persist and return pomodoro-task linkage
- Added a regression test for memo save with task linkage

### Modified: `crates/peekoo-agent-app/`, `crates/peekoo-mcp-server/`, `apps/desktop-tauri/src-tauri/`
- Threaded optional `task_id` through the pomodoro memo save API
- Resolved task titles in the app-facing layers so the pomodoro crate stays task-storage agnostic

### Modified: `apps/desktop-ui/`
- Updated the pomodoro tool client to send `taskId` and receive linked-task history fields
- Added task selection to `PomodoroMemoView`
- Displayed linked task titles in pomodoro history details
- Added frontend tests for the memo client and submit orchestration

### Follow-up adjustment
- Removed the pomodoro-to-task comment side effect after runtime verification issues
- Final behavior: focus memo saves task linkage (`task_id`, `task_title`) in pomodoro history only

## Testing
- `cargo test -p peekoo-pomodoro-app`
- `cargo check -p peekoo-agent-app`
- `cargo check -p peekoo-mcp-server`
- `cargo check -p peekoo-desktop-tauri`
- `bun test src/features/pomodoro/tool-client.test.ts src/views/PomodoroMemoView.test.ts`
- `bun run build`
