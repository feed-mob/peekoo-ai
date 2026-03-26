## 2026-03-26 12:00: feat: task and calendar exact-time notifications with sprite bubble support

**What changed:**
- Added `TaskNotificationScheduler` that fires a notification at the exact `scheduled_start_at` time for tasks
- Wired all task mutations (create, update, toggle, delete, status change) through the scheduler so reminders stay in sync
- Completing or cancelling a task before its start time cancels the pending reminder
- Replaced poll-based Google Calendar reminders (10-minute lead window checked on every sync) with per-event one-shot scheduler entries using `peekoo::schedule::set` with `repeat: false`
- Calendar background sync now works without the panel open — `plugin_init` registers the 5-minute sync schedule, and each sync registers per-event reminder timers via `schedule_event_reminders()`
- Fixed sprite bubble not appearing when system notification fails — `process_plugin_notifications` now attempts both OS notification and sprite bubble independently, only returning an error if neither succeeds
- Added `sourcePlugin` field to `SpriteBubblePayload` schema so task and calendar notifications get stronger visual treatment (dedicated icon, title line, 10s display duration vs 5s default)
- Extracted `sprite-notification-presentation.ts` with `getSpriteBubbleKind` and `getSpriteBubbleDurationMs` for per-source presentation logic
- Removed dead code: `due_notification_ids`, `ReminderState`, `DEFAULT_REMINDER_LEAD_MINUTES`, `prune_notified_ids`, `notified_event_ids` write path

**Why:**
- Google Calendar reminders were only firing when the user manually clicked Refresh or when the background sync happened to run within the 10-minute lead window
- Task notifications were only wired for agent-authored comments and agent status changes, not for scheduled tasks
- Sprite bubble was silently skipped if the OS notification backend failed

**Files affected:**
- `crates/peekoo-agent-app/src/task_notification_scheduler.rs` (new)
- `crates/peekoo-agent-app/src/task_runtime_service.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/lib/sprite-notification-presentation.ts` (new)
- `apps/desktop-ui/src/lib/sprite-notification-presentation.test.ts` (new)
- `apps/desktop-ui/src/types/sprite-bubble.ts`
- `apps/desktop-ui/src/hooks/use-sprite-bubble.ts`
- `apps/desktop-ui/src/views/SpriteView.tsx`
- `apps/desktop-ui/src/components/sprite/SpriteBubble.tsx`
- `plugins/google-calendar/src/lib.rs`
- `plugins/google-calendar/target/wasm32-wasip1/release/google_calendar.wasm`

**PR:** #138
