## 2026-03-26 15:45: fix: pomodoro badge wall and history date filtering timezone issues

**What changed:**
- Fixed timezone mismatch in `PomodoroPanel.tsx` date filtering: replaced `toISOString().split('T')[0]` with local date formatting helper `formatLocalDate()` that uses `getFullYear()`, `getMonth()`, and `getDate()` to avoid UTC conversion
- Fixed daily counter reset logic in `PomodoroAppService::reconcile_runtime_state()`: changed from `chrono::Utc::now()` to `chrono::Local::now()` to use local timezone for date comparison
- Added `publish_badges()` call in `PomodoroAppService::get_status()` to ensure badges refresh on every status poll, not just on state transitions

**Why:**
- The "Today" filter was showing yesterday's records (March 25) when the local date was March 26 because `toISOString()` converts to UTC, which is 8 hours behind China timezone (UTC+8)
- At 15:45 local time (March 26), midnight local time (00:00) becomes 16:00 UTC on March 25, causing the date string to be "2026-03-25" instead of "2026-03-26"
- Backend SQL uses `date(datetime(ended_at, 'localtime'))` which correctly uses local timezone, but frontend was sending UTC-based dates
- The badge wall (completed_focus and completed_breaks counters) wasn't resetting at midnight because `reconcile_runtime_state()` was comparing UTC dates instead of local dates
- This caused the badge wall to accumulate counts across multiple days instead of resetting daily
- Badges weren't refreshing consistently because `get_status()` only called `publish_badges()` indirectly through `refresh_runtime_if_due()` when a timer completed, not on regular status polls

**Impact:**
- "Today" filter now correctly shows March 26 records when local date is March 26
- All date range filters (yesterday, last 7 days, last 30 days) now work correctly across all timezones
- Badge wall counters now reset at local midnight, showing only today's completed sessions
- Peek badges update on every status poll (every 3 seconds), ensuring consistent display

**Files affected:**
- `apps/desktop-ui/src/features/pomodoro/PomodoroPanel.tsx`
- `crates/peekoo-pomodoro-app/src/lib.rs`
