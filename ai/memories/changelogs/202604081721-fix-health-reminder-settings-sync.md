## 2026-04-08 17:21: fix: restore health reminder settings sync

**What changed:**
- Fixed the health reminders plugin to load reminder intervals and enable flags from plugin state so runtime status, schedules, and badges use the same persisted values.
- Fixed the Health Panel script to parse `health_get_status`, map the current reminder payload shape correctly, and refresh countdown/progress immediately after config changes.
- Routed `health-reminders` saves from the generic plugin settings panel through `health_configure` so interval changes reschedule timers instead of only persisting values.
- Added regression coverage for the desktop UI save-path helper.

**Why:**
- Health reminder changes made from the panel or plugin settings could drift out of sync, and the Health Panel countdown/progress was reading the wrong data shape.

**Files affected:**
- `plugins/health-reminders/src/lib.rs`
- `plugins/health-reminders/ui/panel.js`
- `apps/desktop-ui/src/features/plugins/PluginConfigPanel.tsx`
- `apps/desktop-ui/src/features/plugins/plugin-config-save.ts`
- `apps/desktop-ui/src/features/plugins/plugin-config-save.test.ts`
