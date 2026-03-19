## 2026-03-19 17:11 chore: Limit OpenClaw panel refresh triggers

**What changed:**
- Removed the default background auto-refresh timer from `plugins/openclaw-sessions/ui/panel.html`
- Kept session refresh on panel open and manual refresh button clicks
- Added chat-driven panel refresh during active chat polling with throttling (about every 6 seconds)

**Why:**
- Reduce unnecessary background refresh activity while preserving timely updates when users are actively interacting (open, manual refresh, chat)

**Files affected:**
- `plugins/openclaw-sessions/ui/panel.html`
