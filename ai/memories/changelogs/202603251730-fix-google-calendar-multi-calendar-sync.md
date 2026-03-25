# 2026-03-25 17:30: fix: Google Calendar multi-calendar sync

**What changed:**
- Updated `plugins/google-calendar/src/lib.rs` to fetch the Google calendar list from `/users/me/calendarList` and sync events from every readable calendar instead of only `primary`.
- Added stored calendar metadata in plugin state so newly discovered calendars default to `enabled: true` and existing enablement preferences can be preserved for a future settings UI.
- Normalized synced events with the real source calendar name instead of labeling everything as `Primary`.
- Made sync tolerate per-calendar fetch failures and keep successful calendars in the cached snapshot while recording a partial-sync error.
- Added unit tests for readable calendar filtering, default enablement, preference preservation, and calendar-name propagation.

**Why:**
- The plugin only synced the account's default Google calendar, so events from secondary/shared calendars never appeared in Peekoo.
- Persisting calendar metadata now gives us a clean path to add calendar selection in settings later without redesigning sync state.

**Files affected:**
- `plugins/google-calendar/src/lib.rs`
- `ai/memories/changelogs/202603251730-fix-google-calendar-multi-calendar-sync.md`
