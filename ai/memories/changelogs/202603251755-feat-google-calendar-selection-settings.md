# 2026-03-25 17:55: feat: Google Calendar selection settings

**What changed:**
- Added a collapsible calendar settings section to `plugins/google-calendar/ui/panel.html`, `panel.css`, and `panel.js` so users can review and save which Google calendars are enabled for sync.
- Extended `plugins/google-calendar/src/lib.rs` panel snapshots to include stored calendar metadata and added `google_calendar_update_calendar_selection` to persist enabled flags and refresh the agenda.
- Added tests for selection-state helpers and panel rendering of stored calendars/settings controls.
- Added a design note at `docs/plans/2026-03-25-google-calendar-selection-design.md`.

**Why:**
- After fixing multi-calendar sync, users still needed a way to exclude noisy shared calendars while keeping the default of syncing everything readable.

**Files affected:**
- `plugins/google-calendar/src/lib.rs`
- `plugins/google-calendar/peekoo-plugin.toml`
- `plugins/google-calendar/ui/panel.html`
- `plugins/google-calendar/ui/panel.css`
- `plugins/google-calendar/ui/panel.js`
- `apps/desktop-ui/tests/google-calendar-panel.test.js`
- `apps/desktop-ui/tests/google-calendar-panel-runtime.test.js`
- `docs/plans/2026-03-25-google-calendar-selection-design.md`
- `ai/memories/changelogs/202603251755-feat-google-calendar-selection-settings.md`
