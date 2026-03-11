## 2026-03-11 15:07: fix: resync health reminders after system wake

**What changed:**
- Added scheduler wake-drift detection by comparing wall-clock elapsed time with monotonic elapsed time inside `Scheduler::start_with_wake_handler()`
- Added a new scheduler wake callback so hosts can react to suspend/resume without changing the existing fire callback API
- Updated the plugin host registry to dispatch `system:wake` to schedule owners when wake drift is detected
- Updated the health reminders plugin manifest to subscribe to `system:wake`
- Updated the health reminders plugin event handler to call `sync_schedules()` on wake so countdowns are rebuilt from persisted `timer_fire_at:*` wall-clock timestamps
- Added scheduler unit tests covering wake drift detection

**Why:**
- After a machine woke from sleep, in-memory `Instant` deadlines could become stale and all reminders would report `time_remaining_secs = 0`, which rendered as `now`
- The plugin already persisted wall-clock fire times for restart recovery, so wake handling now reuses that source of truth instead of waiting for the next timer cycle to repair itself

**Files affected:**
- `crates/peekoo-scheduler/src/scheduler.rs`
- `crates/peekoo-plugin-host/src/registry.rs`
- `plugins/health-reminders/src/lib.rs`
- `plugins/health-reminders/peekoo-plugin.toml`
