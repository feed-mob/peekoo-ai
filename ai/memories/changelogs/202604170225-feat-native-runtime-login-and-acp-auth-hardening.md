## 2026-04-17 02:25: feat: Native Runtime Login And ACP Auth Hardening

**What changed:**
- Added a native runtime login path for registry-installed ACP agents, starting with `kimi` and `qwen-code`.
- Added provider-specific native login launch generation in `runtime_adapters`.
- Added a separate `launch_native_runtime_login` app/Tauri command that opens a terminal and returns immediately.
- Added `cwd` support to terminal launches so login runs from the installed agent directory under `~/.peekoo/resources/agents/<agent-id>/`.
- Added explicit timeout handling around ACP auth initialize/authenticate/refresh phases so login no longer hangs indefinitely.
- Extended runtime inspection payloads with `nativeLoginCommand` and surfaced native login UI actions in the desktop app.

**Why:**
- Kimi CLI and Qwen ACP login flows were getting stuck with no visible error.
- Registry-installed agents need a reliable provider-native login path that reuses the installed runtime command and install directory.

**Files affected:**
- `ai/plans/2026-04-17-native-runtime-login-and-acp-auth-hardening.md`
- `crates/peekoo-agent-app/src/runtime_adapters/mod.rs`
- `crates/peekoo-agent-app/src/agent_provider_service.rs`
- `crates/peekoo-agent-app/src/agent_provider_commands.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/types/agent-runtime.ts`
- `apps/desktop-ui/src/hooks/useAgentProviders.ts`
- `apps/desktop-ui/src/features/agent-runtimes/ConfigureProviderDialog.tsx`
- `apps/desktop-ui/src/features/agent-runtimes/AgentProviderPanel.tsx`
- `apps/desktop-ui/src/features/chat/ChatPanel.tsx`
- `apps/desktop-ui/src/locales/en.json`
- `apps/desktop-ui/src/locales/zh.json`
