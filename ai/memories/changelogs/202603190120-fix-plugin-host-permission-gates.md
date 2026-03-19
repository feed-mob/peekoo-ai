## 2026-03-19 01:20 fix: Correct plugin host permission gates

**What changed:**
- Fixed host permission gating so `peekoo_notify` requires `notifications`, while `peekoo_schedule_set`, `peekoo_schedule_cancel`, and `peekoo_schedule_get` consistently require `scheduler`
- Removed accidental extra permission checks from plugin logging and custom event emission paths
- Added regression tests covering notification, scheduler, logging, and event permission behavior in the plugin host

**Why:**
- The host was returning misleading runtime errors for plugins like `openclaw-sessions` by checking the wrong capability before sending notifications
- Schedule host functions were inconsistent, which made the permission model harder to trust and debug

**Files affected:**
- `crates/peekoo-plugin-host/src/host_functions.rs`
