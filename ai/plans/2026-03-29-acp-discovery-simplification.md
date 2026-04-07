# ACP Client Discovery and Runtime Simplification Plan

## Overview

Simplify the ACP runtime architecture by adopting Zed-style model discovery from ACP harnesses, removing manual runtime-scoped provider/model management, and separating internal task execution from external chat runtimes.

## Key Decisions

- `pi-acp` (bundled Peekoo ACP) is **NOT** shown in chat runtime selection
- Discovered models are **fetched fresh on demand**, not cached in SQLite
- Scheduler uses bundled `peekoo-agent-acp` for internal tasks
- Chat uses installed external ACP runtimes only
- Custom runtimes get fallback text input for default model if ACP discovery returns nothing

## Architecture

### Before (Current)

```
┌─────────────────────────────────────────────────────────────────┐
│                        Chat Settings                             │
├─────────────────────────────────────────────────────────────────┤
│  Runtime: [pi-acp ▼]                                             │
│  Model:   [claude-sonnet-4-6 ▼]                                  │
│  LLM Provider: [anthropic ▼]                                     │
│  Runtime Provider: [openai ▼]                                    │
│  Runtime Model: [gpt-4.1 ▼]                                      │
└─────────────────────────────────────────────────────────────────┘
                    ↓ Manual DB records
┌─────────────────────────────────────────────────────────────────┐
│                 agent_runtimes                                  │
├─────────────────────────────────────────────────────────────────┤
│  runtime_id: "opencode"                                         │
│  config_json: {...}                                             │
└─────────────────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────────────────┐
│            runtime_llm_providers                                  │
├─────────────────────────────────────────────────────────────────┤
│  id: "uuid", runtime_id: "opencode"                             │
│  provider_id: "openai", api_type: "openai"                        │
└─────────────────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────────────────┐
│               runtime_models                                    │
├─────────────────────────────────────────────────────────────────┤
│  id: "uuid", runtime_id: "opencode"                             │
│  provider_id: "openai", model_id: "gpt-4.1"                       │
└─────────────────────────────────────────────────────────────────┘
```

### After (Target)

```
┌─────────────────────────────────────────────────────────────────┐
│                        Chat Settings                             │
├─────────────────────────────────────────────────────────────────┤
│  Runtime: [OpenCode ▼]  → spawns: npx opencode agent            │
│  Model:   [gpt-4.1 ▼]  ← from ACP new_session response           │
│  Auth:    [Login required ▼]  ← from ACP auth_methods           │
└─────────────────────────────────────────────────────────────────┘
                    ↓ ACP Protocol
┌─────────────────────────────────────────────────────────────────┐
│              NewSessionRequest → NewSessionResponse              │
├─────────────────────────────────────────────────────────────────┤
│  {                                                              │
│    "sessionId": "sess_abc123",                                   │
│    "models": {                                                 │
│      "available_models": [                                     │
│        {"model_id": "gpt-4.1", "name": "GPT-4.1"}                │
│      ],                                                         │
│      "current_model_id": "gpt-4.1"                             │
│    },                                                           │
│    "configOptions": [...]  ← category: "model"                │
│  }                                                              │
└─────────────────────────────────────────────────────────────────┘
                    ↓ Discovered Fresh
┌─────────────────────────────────────────────────────────────────┐
│              Session State (In-Memory)                          │
├─────────────────────────────────────────────────────────────────┤
│  models: [...]                                                  │
│  current_model: "gpt-4.1"                                       │
│  auth_methods: [...]                                           │
└─────────────────────────────────────────────────────────────────┘
```

## Execution Phases

### Phase 1: Runtime Role Split (Estimated: 2-3 hours)

**Files:**
- `crates/peekoo-agent-app/src/application.rs` (add chat vs scheduler separation)
- `crates/peekoo-agent-app/src/agent_provider_service.rs` (add runtime classification)
- `crates/peekoo-agent-app/src/settings/mod.rs` (filter chat runtime catalog)

**Implementation:**
1. Define runtime classification rule:
   - Internal: `pi-acp` / bundled Peekoo ACP
   - External: `opencode`, `claude-code`, `codex`
   - Custom: user-defined external ACP runtime

2. In `application.rs`, separate:
   - `resolve_chat_runtime_config()` - returns only external/custom runtimes
   - `resolve_scheduler_runtime_config()` - returns bundled Peekoo ACP

3. In settings, when building chat catalog:
   - Filter out internal bundled runtime
   - Show only installed external/custom runtimes

**Acceptance Criteria:**
- Chat runtime selector excludes `pi-acp`
- Scheduler continues to use bundled `peekoo-agent-acp`
- `list_agent_runtimes` for chat excludes internal runtime

---

### Phase 2: Real ACP Client for Chat (Estimated: 4-6 hours)

**Files:**
- `crates/peekoo-agent/src/backend/acp.rs` (major rewrite)
- `crates/peekoo-agent/src/service.rs` (consume ACP state)
- `crates/peekoo-agent/src/config.rs` (revisit defaults)

**Implementation:**
1. Implement real ACP protocol in `acp.rs`:
   - stdio JSON-RPC transport
   - Request/response framing
   - Message parsing and validation

2. Implement ACP methods:
   ```rust
   async fn initialize(&mut self) -> Result<InitializeResult>
   async fn new_session(&mut self, request: NewSessionRequest) -> Result<NewSessionResponse>
   async fn prompt(&mut self, request: PromptRequest) -> Result<PromptResponse>
   async fn authenticate(&mut self, request: AuthenticateRequest) -> Result<()>
   async fn set_session_model(&mut self, request: SetSessionModelRequest) -> Result<()>
   ```

3. Parse and retain from `NewSessionResponse`:
   - `models` (legacy SessionModelState)
   - `config_options` (preferred modern approach)
   - Session ID
   - Agent capabilities

4. In `service.rs`, when creating `AgentService`:
   - Use discovered models from ACP session
   - Persist runtime/model context in session store
   - Do NOT fall back to hardcoded catalog

5. In `config.rs`:
   - Change default chat provider behavior
   - Do NOT default to `pi-acp` for chat
   - Allow "no runtime selected" state until user installs external runtime

**Acceptance Criteria:**
- ACP backend can spawn and connect to external runtime
- `new_session` returns real discovered metadata
- Chat prompts go through real ACP protocol, not simulation

---

### Phase 3: Runtime Inspection API (Estimated: 3-4 hours)

**Files:**
- `crates/peekoo-agent-app/src/application.rs` (add inspection flow)
- `crates/peekoo-agent-app/src/agent_provider_commands.rs` (add commands)
- `crates/peekoo-agent-app/src/agent_runtime_commands.rs` (re-exports)
- `apps/desktop-tauri/src-tauri/src/lib.rs` (register Tauri commands)

**Implementation:**
1. Add `inspect_runtime()` in `application.rs`:
   ```rust
   pub fn inspect_runtime(&self, runtime_id: &str) -> Result<RuntimeInspectionResult, String>
   ```

2. Inspection flow:
   - Get runtime config (command, args, env)
   - Spawn runtime process
   - ACP initialize
   - Create temporary session (not persisted)
   - Collect:
     - auth_methods
     - auth_required flag
     - models (from session response)
     - config_options with model category
     - current_model
     - supports_model_switching
   - Kill temporary process

3. Add Tauri commands:
   ```rust
   #[tauri::command]
   async fn inspect_runtime(runtime_id: String, state: State<'_, AgentState>) -> Result<RuntimeInspectionResult, String>
   
   #[tauri::command]
   async fn authenticate_runtime(runtime_id: String, method_id: String, state: State<'_, AgentState>) -> Result<(), String>
   
   #[tauri::command]
   async fn refresh_runtime_capabilities(runtime_id: String, state: State<'_, AgentState>) -> Result<RuntimeInspectionResult, String>
   ```

4. DTO for inspection result:
   ```rust
   pub struct RuntimeInspectionResult {
       pub runtime_id: String,
       pub auth_methods: Vec<AuthMethod>,
       pub auth_required: bool,
       pub discovered_models: Vec<DiscoveredModel>,
       pub current_model_id: Option<String>,
       pub supports_model_selection: bool,
       pub supports_config_options: bool,
   }
   ```

**Acceptance Criteria:**
- Settings can inspect a runtime without creating a chat session
- Model list is fetched fresh from ACP session
- No caching in SQLite - every inspection creates fresh temporary session

---

### Phase 4: Zed-Style Auth Integration (Estimated: 3-4 hours)

**Files:**
- `crates/peekoo-agent/src/backend/acp.rs` (expose auth methods)
- `crates/peekoo-agent-app/src/application.rs` (auth orchestration)
- `apps/desktop-ui/src/hooks/useAgentProviders.ts` (auth hooks)
- `apps/desktop-ui/src/features/agent-runtimes/ConfigureProviderDialog.tsx` (auth UI)
- `apps/desktop-ui/src/features/chat/settings/ChatSettingsPanel.tsx` (runtime auth status)

**Implementation:**
1. In `acp.rs`, expose from initialize response:
   ```rust
   pub fn auth_methods(&self) -> &[AuthMethod]
   pub fn is_auth_required(&self) -> bool
   ```

2. Handle `AuthRequired` error from ACP:
   ```rust
   if err.code == ErrorCode::AuthRequired {
       return Err(AuthError::Required)
   }
   ```

3. Support terminal-auth style flows:
   - If runtime exposes `AuthMethod::Terminal`, show login button
   - On click, spawn terminal task with auth command/args
   - After terminal auth completes, re-inspect runtime

4. In UI, replace manual auth-like config with:
   - Auth status indicator
   - Login/Authenticate button
   - Retry/Refresh action

**Acceptance Criteria:**
- Runtime with auth methods shows login action in UI
- Runtime requiring auth shows actionable state
- Manual provider-specific auth forms removed

---

### Phase 5: Remove Hardcoded/Manual Model Source (Estimated: 2-3 hours)

**Files:**
- `crates/peekoo-agent-app/src/settings/catalog.rs` (stop hardcoding)
- `crates/peekoo-agent-app/src/settings/mod.rs` (use discovery)
- `apps/desktop-ui/src/features/chat/settings/useChatSettings.ts` (fetch fresh)
- `apps/desktop-ui/src/features/chat/settings/ChatSettingsPanel.tsx` (use discovery)

**Implementation:**
1. In `catalog.rs`:
   - Remove `models_for_provider()` function
   - Remove hardcoded model lists for ACP runtimes
   - Keep only custom runtime fallback logic

2. In `settings/mod.rs`:
   - Build provider DTO from:
     - Installed external runtimes
     - Plus fresh inspection result for models
   - Call `inspect_runtime()` when building catalog

3. In `useChatSettings.ts`:
   - When active runtime changes:
     - Call `inspect_runtime()`
     - Use discovered models for model selector
   - Do NOT use hardcoded or DB-cached model lists

4. In `ChatSettingsPanel.tsx`:
   - Remove runtime default provider/model summary from DB
   - Show discovered runtime info instead

**Acceptance Criteria:**
- Standard ACP runtimes use only discovered models
- Hardcoded model lists removed from Rust backend
- No DB queries for manual runtime models

---

### Phase 6: Simplify Runtime Config UI (Estimated: 3-4 hours)

**Files:**
- `apps/desktop-ui/src/features/agent-runtimes/ConfigureProviderDialog.tsx` (major simplification)
- `apps/desktop-ui/src/features/agent-runtimes/ProviderCard.tsx` (update summary)
- `apps/desktop-ui/src/features/agent-runtimes/AgentProviderPanel.tsx` (update actions)

**Implementation:**
1. In `ConfigureProviderDialog.tsx`, remove sections:
   - "LLM Providers" (entire section)
   - "Models" CRUD (entire section)
   - All manual provider/model entry forms

2. Keep and reorganize:
   - **Basic** (visible by default):
     - Runtime status (ready/installing/error)
     - Auth status + login button
     - Discovered model dropdown
     - Refresh models button
     - Test connection button
   
   - **Advanced** (collapsed by default):
     - Environment variables
     - Custom arguments

3. Update `ProviderCard.tsx`:
   - Show concise summary:
     - Status badge (ready / auth-required / installing / error)
     - Selected/discovered model
     - Quick "Login" or "Refresh" action

**Acceptance Criteria:**
- Dialog is materially simpler (≈50% fewer fields)
- No manual provider/model entry for standard ACP runtimes
- Custom runtime still has fallback default model text input

---

### Phase 7: Hook/Type Cleanup (Estimated: 2-3 hours)

**Files:**
- `apps/desktop-ui/src/types/agent-runtime.ts` (schema changes)
- `apps/desktop-ui/src/hooks/useAgentProviders.ts` (hook changes)
- `apps/desktop-ui/src/features/agent-runtimes/` (update imports)

**Implementation:**
1. In `types/agent-runtime.ts`, remove:
   ```typescript
   export const runtimeLlmProviderInfoSchema = ...
   export const runtimeLlmProviderUpsertSchema = ...
   export const runtimeModelUpsertSchema = ...
   ```

2. Add new types:
   ```typescript
   export const runtimeInspectionResultSchema = z.object({
     runtimeId: z.string(),
     authMethods: z.array(authMethodSchema),
     authRequired: z.boolean(),
     discoveredModels: z.array(discoveredModelSchema),
     currentModelId: z.string().optional(),
     supportsModelSelection: z.boolean(),
   });
   
   export const discoveredModelSchema = z.object({
     id: z.string(),
     name: z.string(),
     description: z.string().optional(),
   });
   ```

3. In `useAgentProviders.ts`, remove:
   ```typescript
   listRuntimeProviders
   saveRuntimeProvider
   listRuntimeModels
   saveRuntimeModel
   getRuntimeDefaults  // based on manual DB
   ```

4. Add:
   ```typescript
   const inspectRuntime = useCallback(...)
   const authenticateRuntime = useCallback(...)
   const refreshRuntimeCapabilities = useCallback(...)
   ```

**Acceptance Criteria:**
- No manual provider/model types in frontend
- Hook surface matches simplified backend
- All imports updated

---

### Phase 8: Persistence Cleanup (Estimated: 2-3 hours)

**Files:**
- `crates/peekoo-agent-app/src/agent_provider_service.rs` (remove CRUD)
- `crates/persistence-sqlite/migrations/` (cleanup migration)

**Implementation:**
1. In `agent_provider_service.rs`, remove methods:
   ```rust
   pub fn list_runtime_llm_providers(...)
   pub fn upsert_runtime_llm_provider(...)
   pub fn list_runtime_models(...)
   pub fn upsert_runtime_model(...)
   pub fn get_default_runtime_llm_provider(...)
   pub fn get_default_runtime_model(...)
   ```

2. Keep only:
   ```rust
   pub fn list_runtimes()
   pub fn install_runtime()
   pub fn uninstall_runtime()
   pub fn get_runtime_config()
   pub fn update_runtime_config()
   pub fn add_custom_runtime()
   pub fn remove_custom_runtime()
   pub fn set_default_runtime()
   ```

3. Add cleanup migration (optional, if fully obsolete):
   ```sql
   -- @migrate: alter
   -- @id: 0018_cleanup_runtime_tables
   -- @tolerates: "no such table"
   
   DROP TABLE IF EXISTS runtime_llm_providers;
   DROP TABLE IF EXISTS runtime_models;
   ```

**Acceptance Criteria:**
- `agent_runtimes` is the only runtime persistence table in active use
- No stale provider/model CRUD path remains

---

### Phase 9: End-to-End Validation (Estimated: 4-6 hours)

**Test Matrix:**

| Runtime | Install | Auth | Discovery | Model Selection | Custom Model Fallback |
|---------|---------|------|-------------|-----------------|----------------------|
| opencode | ✅ | API Key | ✅ | ✅ | N/A |
| claude-code | ✅ | Terminal Auth | ✅ | ✅ | N/A |
| codex | ✅ | Agent Login | ✅ | ✅ | N/A |
| custom (with ACP models) | ✅ | Manual | ✅ | ✅ | N/A |
| custom (no ACP models) | ✅ | Manual | ❌ | N/A | ✅ Text field |

**Validation Steps:**
1. Install each runtime
2. Verify inspection returns auth methods
3. Complete auth flow where applicable
4. Verify model discovery works
5. Start chat with selected runtime
6. Verify prompts go through ACP
7. Switch models during chat (if supported)
8. Verify scheduler still uses bundled harness
9. Verify chat never shows bundled harness

**Acceptance Criteria:**
- All standard ACP runtimes work with discovery
- Custom runtime fallback works
- No regression in scheduler behavior
- Chat runtime selector excludes internal runtime

---

## Migration Path

### For Existing Users

1. Keep existing runtime install records
2. Stop reading from obsolete `runtime_llm_providers` / `runtime_models`
3. First time user opens runtime config:
   - Run inspection
   - Show discovered models
   - If no models for custom runtime, show fallback text field

### Breaking Changes

- Manual runtime provider/model configs will be ignored
- Users will need to re-authenticate or re-inspect runtimes
- Chat will no longer default to `pi-acp` - user must install external runtime

---

## Success Metrics

- [ ] Runtime config dialog has < 6 fields (currently has ~15)
- [ ] No manual provider/model CRUD for standard ACP runtimes
- [ ] Model list comes only from ACP discovery
- [ ] Chat never lists `pi-acp` in runtime selector
- [ ] Scheduler still uses bundled `peekoo-agent-acp`
- [ ] All 4 standard ACP runtimes work with discovery
- [ ] Custom runtime fallback works

---

## Implementation Order

1. **Phase 1**: Runtime role split (2-3h)
2. **Phase 2**: Real ACP client (4-6h)
3. **Phase 3**: Runtime inspection API (3-4h)
4. **Phase 4**: Zed-style auth (3-4h)
5. **Phase 5**: Remove hardcoded models (2-3h)
6. **Phase 6**: Simplify runtime config UI (3-4h)
7. **Phase 7**: Hook/type cleanup (2-3h)
8. **Phase 8**: Persistence cleanup (2-3h)
9. **Phase 9**: End-to-end validation (4-6h)

**Total Estimated Time**: 25-35 hours

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| ACP client implementation is complex | Reference Zed's implementation in `/home/richard/0xdev/code/zed` |
| Some ACP runtimes don't support discovery well | Custom runtime fallback handles this |
| Users confused by new model discovery | Good empty states and "Refresh" buttons |
| Breaking change for existing configs | Clear messaging, migration guide |

---

## References

- Zed ACP implementation: `/home/richard/0xdev/code/zed/crates/agent_servers/src/acp.rs`
- Zed model selector: `/home/richard/0xdev/code/zed/crates/acp_thread/src/connection.rs:368-413`
- Zed ACP auth: `/home/richard/0xdev/code/zed/crates/agent_servers/src/acp.rs:48-360`
- Current Peekoo ACP backend: `/home/richard/feedmob/code/peekoo-ai/crates/peekoo-agent/src/backend/acp.rs`
- Current runtime config UI: `/home/richard/feedmob/code/peekoo-ai/apps/desktop-ui/src/features/agent-runtimes/ConfigureProviderDialog.tsx`

---

**Status**: Ready for implementation  
**Created**: 2026-03-29  
**Priority**: High  
**Estimated Duration**: 25-35 hours  
