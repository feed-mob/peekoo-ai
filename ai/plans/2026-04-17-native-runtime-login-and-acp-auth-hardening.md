# Plan: Native Runtime Login And ACP Auth Hardening

## Overview
Peekoo currently relies on ACP `authenticate` as the primary login path for installed runtimes. That breaks down for Kimi CLI and Qwen because ACP auth can hang indefinitely and the app has no first-class way to launch the provider's native login flow from the installed agent under `~/.peekoo/resources/agents/<agent-id>/`.

This plan adds a separate native-login path for registry-installed agents, keeps ACP login as a secondary path, and hardens ACP auth so it fails fast instead of spinning forever.

## Goals
- [ ] Add a native login action for registry-installed runtimes that opens a terminal and runs the installed agent's login command.
- [ ] Keep ACP `authenticate` available, but add explicit timeouts so failures surface instead of hanging.
- [ ] Scope native login to registry-installed agents only for the first iteration.
- [ ] Support Kimi and Qwen first.
- [ ] Reuse installed agent metadata and install directories under `~/.peekoo/resources/agents/<agent-id>/`.

## Design

### Approach
Use a dual-path auth model:

1. ACP auth remains available through the existing `authenticate_runtime` command.
2. Native login is exposed through a new app/backend command that resolves the installed runtime, builds a provider-specific login command, and asks Tauri to open it in a real terminal.
3. Runtime inspection remains ACP-based for now; after native login the user refreshes capabilities and ACP should report `authRequired: false` if the native login succeeded.

This keeps the first iteration minimal while solving the observed user problem.

### Components
- `runtime_adapters`: add native login launch/manual command support for installed runtimes.
- `agent_provider_service`: resolve installed runtime metadata and install directories.
- `agent_provider_commands`: add ACP auth timeouts and a new native login command.
- `application`: expose native login command to Tauri.
- `desktop-tauri`: launch native login terminals using the existing terminal launcher.
- `desktop-ui`: show native login actions in runtime configuration for supported runtimes.

### Native Login Flow
1. User clicks `Open Terminal to Login`.
2. Frontend invokes a new Tauri command for native runtime login.
3. App layer resolves:
   - installed runtime command and args from DB metadata
   - install dir from `~/.peekoo/resources/agents/<agent-id>/`
   - provider-specific login command from the runtime adapter
4. Tauri opens a terminal with that command and returns immediately.
5. UI shows a non-blocking success message and asks the user to refresh.

### ACP Auth Hardening
Wrap each `authenticate_runtime` stage in a timeout:
- backend initialize
- ACP authenticate
- refresh session capabilities

Return targeted errors on timeout so the UI clears the spinner and exposes the fallback path.

## Implementation Steps

1. **Backend tests first**
   - Add adapter tests for native login command generation.
   - Add command-layer tests for registry-installed native login resolution.
   - Add auth timeout tests for ACP authenticate.

2. **Runtime adapter support**
   - Add native login helpers to the adapter trait.
   - Implement Kimi and Qwen native login support.
   - Keep unsupported runtimes returning `None` cleanly.

3. **Service and command plumbing**
   - Add install-dir resolution for registry-installed runtimes.
   - Add native login command DTOs/result types.
   - Add ACP auth timeouts.

4. **Tauri transport**
   - Add a new command to trigger native login.
   - Reuse the existing terminal launcher and make it work with the new launch DTO.

5. **Frontend UI**
   - Add `Open Terminal to Login`, `Copy Login Command`, and `Refresh Status` controls.
   - Prefer native login for supported runtimes.
   - Keep ACP login available as a secondary path.

6. **Verification**
   - Run targeted Rust tests for `peekoo-agent-app`.
   - Run targeted frontend tests for runtime auth state/UI.

## Files To Modify
- `crates/peekoo-agent-app/src/runtime_adapters/mod.rs`
- `crates/peekoo-agent-app/src/agent_provider_service.rs`
- `crates/peekoo-agent-app/src/agent_provider_commands.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/hooks/useAgentProviders.ts`
- `apps/desktop-ui/src/features/agent-runtimes/ConfigureProviderDialog.tsx`
- `apps/desktop-ui/src/types/agent-runtime.ts`
- `apps/desktop-ui/src/locales/en.json`
- `apps/desktop-ui/src/locales/zh.json`

## Testing Strategy
- Rust unit tests for runtime adapter launch generation.
- Rust command/service tests for registry-installed native login resolution.
- Rust tests proving ACP auth timeouts fail fast.
- Frontend tests for auth-state rendering and new native login affordances.

## Open Questions
- Confirm the exact native login commands for installed Kimi and Qwen agents.
- Decide whether future iterations should merge native-auth detection into inspection payloads or keep refresh ACP-only.
