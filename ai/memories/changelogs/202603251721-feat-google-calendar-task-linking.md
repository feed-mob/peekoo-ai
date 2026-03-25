## 2026-03-25 17:21: feat: add Google Calendar task linking and meeting actions

**What changed:**
- Added Google Calendar plugin tools and state for linking calendar events to tasks, creating tasks from events, unlinking stale links, and tracking created vs linked status.
- Upgraded plugin notifications and sprite bubbles to support action URLs, then used that for Join meeting actions and richer upcoming-event reminders.
- Moved task-linking UX fully into the Google Calendar plugin panel with create/link flows, linked-task viewing, unlink support, success feedback, and task resync safeguards for schedule and description fields.
- Added plugin-host reload fallback so newly added plugin tools recover from stale loaded manifests instead of failing permanently with tool-not-found errors.

**Why:**
- Let users connect calendar events with tasks without coupling core task UI to an optional plugin.
- Match expected calendar behavior with actionable meeting links and clearer event-to-task state.
- Reduce plugin iteration friction by recovering automatically when a plugin manifest adds new tools.

**Files affected:**
- `plugins/google-calendar/src/lib.rs`
- `plugins/google-calendar/peekoo-plugin.toml`
- `plugins/google-calendar/ui/panel.html`
- `plugins/google-calendar/ui/panel.css`
- `plugins/google-calendar/ui/panel.js`
- `crates/peekoo-plugin-host/src/registry.rs`
- `crates/peekoo-plugin-host/src/tools.rs`
- `crates/peekoo-plugin-sdk/src/notify.rs`
- `crates/peekoo-plugin-sdk/src/host_fns.rs`
- `crates/peekoo-notifications/src/service.rs`
- `crates/peekoo-agent-app/src/plugin.rs`
- `apps/desktop-ui/src/components/sprite/SpriteBubble.tsx`
- `apps/desktop-ui/tests/google-calendar-panel.test.js`
- `apps/desktop-ui/tests/google-calendar-panel-runtime.test.js`
