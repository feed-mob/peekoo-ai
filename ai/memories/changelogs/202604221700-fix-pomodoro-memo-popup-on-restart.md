---
title: "fix: stale pomodoro sessions and old memos triggering popup on restart"
date: 2026-04-22
author: opencode
tags: [fix, pomodoro, memo, app-restart]
---

## Summary

Fixed two issues causing the Focus Memo popup to appear unexpectedly after app restart, even when the previous focus session was not completed.

## Problem

1. **Stale Running sessions auto-completed on restart**: If a focus session was Running when the app exited (e.g. app crash, or the user ignored the pause-on-exit path), `reconcile_runtime_state()` would detect the expired timer and call `status.complete()`, recording it as **Completed** with `memo_requested=true`. This incremented `completed_focus` and triggered the memo popup.

2. **Old pending memos resurfaced forever**: The frontend's `usePomodoroWatcher` scanned the last 12 history entries on every app launch for any work cycle with `memo_requested=true` and no memo. This meant a focus session completed days ago could still prompt for a memo on every restart.

## Solution

### Backend (`crates/peekoo-pomodoro-app`)

- In `reconcile_runtime_state()`, when a stale `Running` session has expired (`time_remaining_secs == 0`), call `finish()` instead of `complete()`. This records the session as **Cancelled** — no `memo_requested`, no `completed_focus` increment.
- The scheduler callback `complete_due_session()` remains unchanged; live timer fires still correctly record **Completed**.

### Frontend (`apps/desktop-ui`)

- `hasPendingFocusMemo()` now also checks `entry.outcome === "completed"` as a defensive guard.
- `findLatestPendingFocusMemo()` now filters to only entries whose `ended_at` is within the last **30 minutes**, preventing ancient pending memos from resurfacing.

### Tests

- Added `stale_running_session_on_init_is_recorded_as_cancelled` in `peekoo-pomodoro-app`: simulates a stale Running session in the DB at startup and verifies it is recorded as Cancelled with `completed_focus` unchanged.

## Files Changed

- `crates/peekoo-pomodoro-app/src/lib.rs` — Stale session handling in `reconcile_runtime_state()`, plus new test
- `apps/desktop-ui/src/hooks/use-pomodoro-watcher.ts` — Outcome guard and 30-minute recency filter

## Testing

- `cargo test` — 359 tests passed (all suites)
- `cd apps/desktop-ui && tsc --noEmit` — frontend type-check passed

## Related

- Builds on previous fix: `202604221600-fix-pomodoro-timer-on-app-exit.md` (pause on exit)
