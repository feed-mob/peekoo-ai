## 2026-04-21 12:00: fix: Windows ACP login terminal spam

**What changed:**
- Added a Windows-only background spawn configuration in ACP backend process startup so ACP inspection/auth helper processes do not open visible console windows.
- Removed automatic post-login polling in runtime configuration dialog that repeatedly refreshed capabilities every 5 seconds.
- Added a short terminal-launch cooldown in the dialog to prevent rapid repeated login launches from repeated clicks.

**Why:**
- Kimi/Qwen login on Windows could trigger multiple terminal windows due to repeated background ACP inspections and auth status checks.
- The login flow should open only the user-triggered terminal, while background ACP checks remain invisible.

**Files affected:**
- `crates/peekoo-agent/src/backend/acp.rs`
- `apps/desktop-ui/src/features/agent-runtimes/ConfigureProviderDialog.tsx`
