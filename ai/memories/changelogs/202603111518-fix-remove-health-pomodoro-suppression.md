## 2026-03-11 15:18: fix: remove pomodoro suppression from health reminders

**What changed:**
- Removed pomodoro event subscriptions from the health reminders plugin manifest
- Removed `suppress_during_pomodoro` config and all pomodoro-specific state from the health reminders WASM plugin
- Simplified health reminder status payloads so they only report reminder config and reminder timers
- Removed pomodoro-specific paused messaging from the health reminders panel UI
- Updated manifest example coverage and peek badge docs to match the new behavior

**Why:**
- The product requirements describe pomodoro and health reminders as separate features and do not require health reminders to pause during focus sessions
- Keeping health reminders active during pomodoro matches the intended product behavior and removes extra state/config complexity

**Files affected:**
- `plugins/health-reminders/src/lib.rs`
- `plugins/health-reminders/peekoo-plugin.toml`
- `plugins/health-reminders/ui/panel.js`
- `apps/desktop-ui/tests/health-reminders-panel.test.js`
- `crates/peekoo-plugin-host/src/manifest.rs`
- `ai/memories/docs/peek-badge-events.md`
