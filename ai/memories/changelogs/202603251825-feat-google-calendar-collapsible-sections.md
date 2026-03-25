# 2026-03-25 18:25: feat: Google Calendar collapsible account and settings sections

**What changed:**
- Updated `plugins/google-calendar/ui/panel.html` to make the Connected account section collapsible and keep Settings as a collapsible section.
- Updated `plugins/google-calendar/ui/panel.js` to default the account section to collapsed when connected and expanded when disconnected, while preserving the settings toggle behavior.
- Added panel tests covering the new account toggle markup and runtime collapsed/expanded behavior.

**Why:**
- The Google Calendar panel became too tall once multi-calendar settings were added, so collapsing setup-oriented sections keeps the agenda view cleaner.

**Files affected:**
- `plugins/google-calendar/ui/panel.html`
- `plugins/google-calendar/ui/panel.css`
- `plugins/google-calendar/ui/panel.js`
- `apps/desktop-ui/tests/google-calendar-panel.test.js`
- `apps/desktop-ui/tests/google-calendar-panel-runtime.test.js`
- `ai/memories/changelogs/202603251825-feat-google-calendar-collapsible-sections.md`
