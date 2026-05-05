## 2026-05-05 12:00: feat: Hermes ACP Runtime

**What changed:**
- Added PATH-based Hermes Agent ACP runtime detection.
- Added Available Runtimes guidance with Hermes install docs and install command.
- Added Hermes' official logo for installed runtime and active runtime displays.
- Added backend and frontend tests for Hermes detection and guidance visibility.

**Why:**
- Users can use Hermes as an ACP runtime after installing it locally, without Peekoo managing the Python installation.

**Files affected:**
- `crates/peekoo-agent-app/src/agent_provider_service.rs`
- `apps/desktop-ui/src/features/agent-runtimes/AgentProviderPanel.tsx`
- `apps/desktop-ui/src/features/agent-runtimes/ProviderCard.tsx`
- `apps/desktop-ui/src/features/agent-runtimes/HermesInstallGuidanceCard.tsx`
- `apps/desktop-ui/src/features/agent-runtimes/hermes-install-guidance.ts`
- `apps/desktop-ui/src/features/agent-runtimes/hermes-install-guidance.test.ts`
- `apps/desktop-ui/src/features/agent-runtimes/runtime-icon-url.ts`
- `apps/desktop-ui/src/features/agent-runtimes/runtime-icon-url.test.ts`
- `apps/desktop-ui/src/locales/*.json`
