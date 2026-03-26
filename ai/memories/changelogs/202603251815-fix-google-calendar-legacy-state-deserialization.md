# 2026-03-25 18:15: fix: Google Calendar legacy state deserialization

**What changed:**
- Added a serde default for the internal `calendar_id` field on stored Google Calendar events in `plugins/google-calendar/src/lib.rs`.
- Added a regression test proving legacy persisted plugin state without `calendarId` still deserializes.

**Why:**
- Existing users had persisted Google Calendar plugin state from before `calendar_id` was introduced, causing the WASM runtime to fail with `missing field \`calendarId\`` during state deserialization.

**Files affected:**
- `plugins/google-calendar/src/lib.rs`
- `ai/memories/changelogs/202603251815-fix-google-calendar-legacy-state-deserialization.md`
