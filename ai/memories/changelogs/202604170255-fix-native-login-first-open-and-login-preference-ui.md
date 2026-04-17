## 2026-04-17 02:55: fix: Native login first-open visibility and login preference UI

**What changed:**
- Invalidated stale cached runtime inspections for native-preferred runtimes when cached payloads are missing `nativeLoginCommand` or `preferredLoginMethod`.
- Updated the configure dialog so native-preferred runtimes do not auto-expand ACP login fallback on first open.
- Kept ACP login available behind advanced login options for runtimes whose adapters mark native login as preferred.

**Why:**
- Kimi and Qwen native login was only appearing after a manual `Refresh Status` because older cached inspection records predated the new login-preference fields.
- Native-preferred runtimes should present native login as the default action immediately, with ACP fallback remaining secondary.

**Files affected:**
- `crates/peekoo-agent-app/src/agent_provider_service.rs`
- `apps/desktop-ui/src/features/agent-runtimes/ConfigureProviderDialog.tsx`
