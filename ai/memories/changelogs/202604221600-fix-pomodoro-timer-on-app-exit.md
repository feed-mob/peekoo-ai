---
title: "fix: pause pomodoro timer on app exit to prevent auto-completion on restart"
date: 2026-04-22
author: opencode
tags: [fix, pomodoro, timer, app-lifecycle, tauri]
---

## Summary

When the user quits the app via the tray menu, the pomodoro timer now pauses instead of continuing to run in the background. This prevents the timer from auto-completing while the app is closed and the memo window from popping up on the next app launch.

## Problem

- The pomodoro timer state is persisted in SQLite and survives app restarts
- On app startup, `reconcile_runtime_state()` checks if a running timer expired while the app was closed
- If the timer had expired, it auto-completes the session, inserts a history record, and potentially auto-advances to the next session
- The frontend's `usePomodoroWatcher` detects the newly completed session and opens the memo window
- This caused the pomodoro timer and memo window to reappear unexpectedly after reopening the app
- Health reminders had similar persistence but are passive periodic notifications where this behavior is correct

## Solution

- Added `RunEvent::ExitRequested` handler in the Tauri app lifecycle
- On true app exit (tray → Quit), call `AgentApplication::pause_pomodoro_on_exit()`
- `pause_pomodoro_on_exit()` calls `pomodoro.pause()` and gracefully ignores "cannot pause unless running" errors
- The timer is paused and persisted to the database before the app process terminates
- On restart, the timer remains paused and no auto-completion or memo popup occurs
- Window close (X button) still only hides the window to tray — timer continues running as before

## Files Changed

- `apps/desktop-tauri/src-tauri/src/lib.rs` — Added `RunEvent::ExitRequested` handler; switched from `.run()` to `.build().run()`
- `crates/peekoo-agent-app/src/application.rs` — Added `pause_pomodoro_on_exit()` method

## Testing

- `just check` — compiles successfully
- `cargo test -p peekoo-pomodoro-app -p peekoo-agent-app --lib` — 112 tests passed

## Related

- GitHub Issue: #268
- Health reminders plugin behavior is unchanged (correctly persists across restarts)
