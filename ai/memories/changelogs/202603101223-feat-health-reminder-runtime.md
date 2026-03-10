## 2026-03-10 12:23: feat: scheduler-backed health reminder runtime and plugin config host

**What changed:**
- Added `peekoo-scheduler` for background schedule execution and `peekoo-notifications` for DND-aware notification delivery
- Reworked `peekoo-plugin-host` to expose scheduler/config host functions and to route plugin notifications outside the deferred event bus
- Updated `peekoo-agent-app` and Tauri commands to start the scheduler, flush notification messages, bridge pomodoro lifecycle events, and expose plugin config and DND commands
- Rewrote the `health-reminders` plugin to use host schedules instead of `timer:tick`, added manifest config schema, and refreshed its panel UI
- Added a generated plugin settings panel in the desktop UI so installed plugins can expose manifest-defined config fields and a global DND toggle

**Why:**
- The original reminder plugin never fired because the old tick timer path was dead and notification draining depended on tool invocations
- Scheduling and notifications needed clearer ownership so reminders can run in the background without event bus coupling
- The product issue requires configurable health reminders, pomodoro suppression, and DND support aligned with the original design

**Files affected:**
- `Cargo.toml`
- `Cargo.lock`
- `crates/peekoo-scheduler/Cargo.toml`
- `crates/peekoo-scheduler/src/lib.rs`
- `crates/peekoo-scheduler/src/scheduler.rs`
- `crates/peekoo-scheduler/tests/scheduler.rs`
- `crates/peekoo-notifications/Cargo.toml`
- `crates/peekoo-notifications/src/lib.rs`
- `crates/peekoo-notifications/src/service.rs`
- `crates/peekoo-notifications/tests/service.rs`
- `crates/peekoo-plugin-host/Cargo.toml`
- `crates/peekoo-plugin-host/src/config.rs`
- `crates/peekoo-plugin-host/src/host_functions.rs`
- `crates/peekoo-plugin-host/src/lib.rs`
- `crates/peekoo-plugin-host/src/manifest.rs`
- `crates/peekoo-plugin-host/src/registry.rs`
- `crates/peekoo-agent-app/Cargo.toml`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-agent-app/src/plugin.rs`
- `apps/desktop-tauri/src-tauri/Cargo.toml`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/features/plugins/PluginConfigPanel.tsx`
- `apps/desktop-ui/src/features/plugins/PluginList.tsx`
- `apps/desktop-ui/src/types/plugin.ts`
- `plugins/health-reminders/peekoo-plugin.toml`
- `plugins/health-reminders/src/lib.rs`
- `plugins/health-reminders/ui/panel.html`
- `plugins/health-reminders/ui/panel.css`
- `plugins/health-reminders/ui/panel.js`
