# 2026-03-25 18:05: fix: Google Calendar stable calendar-id filtering

**What changed:**
- Added an internal `calendar_id` field to stored Google Calendar events in `plugins/google-calendar/src/lib.rs`.
- Updated event normalization to capture both `calendar_id` and `calendar_name` from the source calendar.
- Switched calendar-selection filtering to use `calendar_id` instead of matching by display name.
- Kept `calendar_id` internal by introducing DTO serialization that omits it from panel/tool responses.
- Added tests for stable filtering by calendar id and for omitting the internal field from serialized outputs.

**Why:**
- Calendar names are not guaranteed to be unique, so filtering by display name could disable or retain the wrong events when multiple calendars share the same label.

**Files affected:**
- `plugins/google-calendar/src/lib.rs`
- `ai/memories/changelogs/202603251805-fix-google-calendar-stable-calendar-id-filtering.md`
