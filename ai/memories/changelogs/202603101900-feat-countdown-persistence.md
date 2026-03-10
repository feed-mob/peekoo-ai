## 2026-03-10 19:00: feat: persist health reminder countdowns across app restarts

**What changed:**
- Added `delay_secs: Option<u64>` parameter to `Scheduler::set()` so the first fire can differ from the repeat interval
- Updated `host_schedule_set` host function to pass optional `delay_secs` from plugin JSON input to the scheduler
- Added `ScheduleSetRequest.delay_secs` optional field in the health-reminders WASM plugin
- Plugin now persists wall-clock "fire at" timestamps (`timer_fire_at:<key>`) and interval (`timer_interval:<key>`) to SQLite via `peekoo_state_set` whenever a timer is set or fires
- `sync_schedules()` (called on init) reads persisted timestamps and computes remaining delay, resuming timers where they left off instead of resetting to full interval
- `handle_schedule_fired()` persists fresh timestamps after each auto-repeat fire
- Added 3 new scheduler integration tests for delay_secs behavior (override, None fallback, zero immediate fire)

**Why:**
- Previously, all countdown timers reset to their full configured interval on every app restart, losing progress (e.g., 5 min left on a 45 min water timer would reset to 45 min)
- Users expect timers to resume where they left off after closing and reopening the app
- If a timer was due while the app was closed, the missed reminder is skipped and the delay is set to the remaining time in the next cycle (e.g., 45-min timer overdue by 2 min resumes with 43 min). Other plugins can opt into immediate firing via `fire_if_overdue: true`.

**Edge cases handled:**
- First-ever launch (no stored timestamps): uses full interval
- App closed longer than interval: skips missed reminder, computes position in next cycle via `overdue % interval`
- Interval changed while app was closed: ignores stored timestamp, starts fresh with new interval
- Dismiss resets timer: persists new full-interval timestamp
- Pomodoro suppression: timestamps become stale during suppression but are overwritten when schedules resume

**Files affected:**
- `crates/peekoo-scheduler/src/scheduler.rs` — added `delay_secs` parameter to `set()`
- `crates/peekoo-scheduler/tests/scheduler.rs` — 3 new tests, 2 updated for new signature
- `crates/peekoo-plugin-host/src/host_functions.rs` — pass `delay_secs` through to scheduler
- `plugins/health-reminders/src/lib.rs` — timestamp persistence, remaining delay computation, updated schedule helpers
