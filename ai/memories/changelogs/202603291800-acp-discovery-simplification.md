# ACP Runtime Discovery and Simplification Implementation

## Overview

This implementation completes Phases 1-8 of the ACP Runtime Discovery and Simplification plan. The goal was to:

1. Separate internal task scheduler runtimes (bundled `pi-acp`) from external chat-visible runtimes
2. Implement real ACP client using the `agent-client-protocol` crate instead of stub implementations  
3. Discover models from ACP `new_session` response rather than hardcoded/manual DB entries
4. Simplify the runtime configuration UI by removing manual provider/model CRUD
5. Implement Zed-style auth flow

## Summary of Changes

### Phase 1: Runtime Role Split ✓

**Key Changes:**
- Added `is_chat_visible()` method to `RuntimeInfo` (bundled = internal only)
- Updated `catalog_from_runtimes()` to filter by `is_chat_visible()`
- Added `default_chat_runtime()` and `first_chat_runtime()` methods
- Updated install logic to only auto-set chat-visible runtimes as default

**Files Modified:**
- `crates/peekoo-agent-app/src/agent_provider_service.rs`
- `crates/peekoo-agent-app/src/settings/mod.rs`
- `crates/peekoo-agent-app/src/application.rs`

### Phase 2: Real ACP Client ✓

**Key Changes:**
- Complete rewrite of `acp.rs` using `ClientSideConnection` from ACP crate
- Implements `Agent` trait methods: `initialize`, `new_session`, `prompt`, `cancel`
- Discovers models from ACP session `config_options` and legacy `modes`
- Solved `!Send` future problem using `spawn_local` wrapper with channels
- Handles MCP capability detection (`http || sse` flags)

**Files Modified:**
- `crates/peekoo-agent/src/backend/acp.rs` (rewritten from scratch)

### Phase 3: Runtime Inspection API ✓

**Key Changes:**
- Added DTOs: `DiscoveredModelInfo`, `AuthMethodInfo`, `RuntimeInspectionResult`
- Implemented `inspect_runtime()` method in `AgentProviderService`
- Creates temporary ACP session to discover capabilities without full chat
- Made `get_runtime_command()` public for inspection use

**Files Modified:**
- `crates/peekoo-agent-app/src/agent_provider_service.rs`
- `crates/peekoo-agent-app/src/agent_provider_commands.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`

### Phase 4: Zed-style Auth Integration ✓

**Key Changes:**
- Added `authenticate()` method to `AcpBackend`
- Implemented `authenticate_runtime()` in provider commands
- Added `is_auth_required()` check based on auth methods
- Auth commands wired through to Tauri backend

**Files Modified:**
- `crates/peekoo-agent/src/backend/acp.rs`
- `crates/peekoo-agent-app/src/agent_provider_commands.rs`

### Phase 5: Remove Hardcoded/Manual Model Source ✓

**Key Changes:**
- Removed hardcoded model lists from `catalog.rs`
- `models_for_provider()` now returns empty slice
- `provider_catalog()` now returns empty vec
- Updated `catalog_from_runtimes()` to use async inspection for models
- Models discovered fresh from ACP, not cached in DB

**Files Modified:**
- `crates/peekoo-agent-app/src/settings/catalog.rs`
- `crates/peekoo-agent-app/src/settings/mod.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`

### Phase 6: Simplify Runtime Config UI ✓

**Key Changes:**
- Rewrote `ConfigureProviderDialog.tsx` with simplified UI (~50% fewer fields)
- Removed LLM Providers and Models CRUD sections
- Added Basic section with status badge, auth status, model dropdown, refresh button
- Added Advanced section (collapsible) with env vars and custom args
- Updated `ProviderCard.tsx` with auth status badge and model display
- Created `collapsible.tsx` UI component
- Installed `@radix-ui/react-collapsible` dependency
- Added CSS animations for collapsible

**Files Modified:**
- `apps/desktop-ui/src/features/agent-runtimes/ConfigureProviderDialog.tsx` (rewritten)
- `apps/desktop-ui/src/features/agent-runtimes/ProviderCard.tsx` (updated)
- `apps/desktop-ui/src/features/agent-runtimes/AgentProviderPanel.tsx`
- `apps/desktop-ui/src/components/ui/collapsible.tsx` (new)
- `apps/desktop-ui/src/types/agent-runtime.ts`
- `apps/desktop-ui/src/hooks/useAgentProviders.ts`
- `apps/desktop-ui/src/index.css`

### Phase 7: Hook/Type Cleanup ✓

**Key Changes:**
- Removed legacy type exports from `agent-runtime.ts`
- Removed legacy methods from `useAgentProviders.ts`:
  - `listRuntimeProviders`
  - `saveRuntimeProvider`
  - `listRuntimeModels`
  - `saveRuntimeModel`
  - `getRuntimeDefaults`
- Moved legacy types to `agent-runtime-legacy.ts` for backward compatibility

**Files Modified:**
- `apps/desktop-ui/src/types/agent-runtime.ts`
- `apps/desktop-ui/src/types/agent-runtime-legacy.ts` (new)
- `apps/desktop-ui/src/hooks/useAgentProviders.ts`

### Phase 8: Persistence Cleanup ✓

**Key Changes:**
- Removed legacy DTO types: `RuntimeLlmProviderInfo`, `RuntimeModelInfo`, etc.
- Removed legacy CRUD methods from service and application
- Dropped obsolete tables via migration:
  - `runtime_llm_providers`
  - `runtime_models`
- Updated Tauri command handlers to remove legacy commands
- Fixed test that checked for dropped tables

**Files Modified:**
- `crates/peekoo-agent-app/src/agent_provider_service.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/runtime_adapters/mod.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/persistence-sqlite/migrations/202603291800_cleanup_runtime_tables.sql` (new)
- `crates/persistence-sqlite/src/lib.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`

## Technical Highlights

### ACP Non-Send Future Solution

The ACP crate uses `#[async_trait(?Send)]` which produces non-Send futures. To satisfy the `AgentBackend` trait's Send requirement, we implemented a channel-based wrapper:

```rust
// Commands sent via mpsc::channel
// Results returned via oneshot::channel
// ACP operations run in spawn_local task
```

### Model Discovery Flow

1. User opens runtime config dialog
2. Frontend calls `inspect_runtime()`
3. Backend spawns ACP process temporarily
4. Performs `initialize` and `new_session`
5. Extracts models from `config_options` (modern) or `modes` (legacy)
6. Returns `RuntimeInspectionResult` with discovered models
7. Frontend displays models in dropdown
8. Process is killed after inspection

### Auth Flow

1. Inspection returns `auth_methods` list
2. UI shows "Login Required" badge if auth needed
3. User clicks "Login" button
4. Frontend calls `authenticate_runtime(runtime_id, method_id)`
5. Backend creates temporary ACP backend and calls `authenticate()`
6. ACP handles the actual auth flow (API key, terminal, etc.)

## Phase 9: Manual Testing Required

The following end-to-end validation must be done manually:

### Test Matrix

| Runtime | Install | Auth | Discovery | Model Selection | Custom Fallback |
|---------|---------|------|-----------|-----------------|-----------------|
| opencode | Manual | API Key | Test | Test | N/A |
| claude-code | Manual | Terminal | Test | Test | N/A |
| codex | Manual | Agent Login | Test | Test | N/A |
| custom (ACP models) | Manual | Manual | Test | Test | N/A |
| custom (no ACP) | Manual | Manual | N/A | N/A | Test text field |

### Validation Steps

1. **Install Runtime**
   - Install external runtime (opencode/claude-code/codex)
   - Verify appears in "Installed Runtimes" section

2. **Inspect Runtime**
   - Open configure dialog
   - Verify inspection runs automatically
   - Check that auth methods appear if required

3. **Auth Flow**
   - If auth required, click Login button
   - Complete auth (API key entry, terminal, etc.)
   - Verify inspection refreshes after auth

4. **Model Discovery**
   - Verify models appear in dropdown
   - Test refresh button
   - Select different model

5. **Chat Testing**
   - Set runtime as default
   - Start new chat
   - Verify prompts go through ACP
   - Test model switching if supported

6. **Scheduler Verification**
   - Create a task with agent assignment
   - Verify scheduler still uses bundled `peekoo-agent-acp`
   - Verify internal runtime not shown in chat selector

7. **Custom Runtime Testing**
   - Add custom runtime
   - Test with ACP-compatible agent (should discover models)
   - Test with non-ACP agent (should show fallback text field)

### Acceptance Criteria

- [ ] All standard ACP runtimes work with discovery
- [ ] Custom runtime fallback works
- [ ] No regression in scheduler behavior
- [ ] Chat runtime selector excludes internal runtime
- [ ] Auth flows complete successfully
- [ ] Model switching works in chat

## Breaking Changes

1. **Manual provider/model configs are ignored** - Previously configured runtime LLM providers and models in the database will no longer be read. They will be discovered fresh from ACP.

2. **Users must re-authenticate** - Auth state is now handled via ACP protocol. Users may need to re-authenticate with their runtimes.

3. **Chat defaults changed** - Chat will no longer default to `pi-acp`. Users must install an external runtime.

## Migration for Users

1. Keep existing runtime install records (agent_runtimes table)
2. First time opening runtime config, inspection runs automatically
3. If auth required, complete auth flow
4. Select desired model from discovered list
5. Save configuration

## Files Created

- `crates/peekoo-agent/src/backend/acp.rs` (rewritten)
- `apps/desktop-ui/src/components/ui/collapsible.tsx`
- `apps/desktop-ui/src/types/agent-runtime-legacy.ts`
- `crates/persistence-sqlite/migrations/202603291800_cleanup_runtime_tables.sql`

## Statistics

- **Lines Added**: ~2,500 (new ACP implementation, UI components)
- **Lines Removed**: ~1,800 (legacy code, hardcoded models, CRUD)
- **Files Modified**: 25+
- **New Files**: 4
- **Tests Passing**: 67 (peekoo-agent-app) + 10 (persistence-sqlite)

## Next Steps

1. Build desktop application
2. Install external ACP runtimes for testing
3. Run through Phase 9 validation matrix
4. Address any issues found during manual testing
5. Update documentation for users

---

**Implementation Date**: 2026-03-29
**Status**: Phases 1-8 Complete, Phase 9 (Manual Testing) Pending
**Branch**: feature/acp-discovery-simplification
