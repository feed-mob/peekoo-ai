# ACP Runtime Architecture Implementation Plan

**Date:** 2026-03-28  
**Status:** In Progress  
**Goal:** Separate ACP agent runtimes from LLM providers, implement hybrid config without ACP RFDs

## Overview

This plan restructures Peekoo's agent architecture to correctly model:
- **ACP Agent Runtime**: The harness/CLI that speaks ACP (codex-acp, claude-code-acp, opencode, pi-acp)
- **LLM Provider**: The backend API target (openai, anthropic, openrouter, azure, etc.)
- **Model**: Specific model within that provider

## Scope

**In scope:**
- ACP core protocol only (initialize, session/new, session/load, prompts)
- ACP Registry as optional metadata source
- Peekoo-managed provider/model/auth configuration
- Hybrid UI supporting both known and custom ACP runtimes

**Out of scope:**
- ACP RFD methods (providers/list, providers/set, providers/disable)
- Protocol-level auth methods discovery
- Model discovery via ACP (use runtime-specific adapters)

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Peekoo Application                    │
│  ┌───────────────────────────────────────────────────┐  │
│  │         ACP Runtime Management Layer              │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────┐ │  │
│  │  │codex-acp│ │claude   │ │opencode │ │custom  │ │  │
│  │  │adapter  │ │-code-acp│ │adapter  │ │runtime │ │  │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └───┬────┘ │  │
│  │       │           │           │          │       │  │
│  │       └───────────┴───────────┴──────────┘       │  │
│  │                   ACP Core Transport              │  │
│  └───────────────────────────────────────────────────┘  │
│                          │                               │
│  ┌───────────────────────┼───────────────────────────┐   │
│  │     LLM Provider Config (Peekoo-managed)        │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────┐ │   │
│  │  │openai   │ │anthropic│ │openrouter│ │azure   │ │   │
│  │  │config   │ │config   │ │config    │ │config  │ │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └────────┘ │   │
│  └────────────────────────────────────────────────────┘   │
│                          │                               │
│  ┌───────────────────────┼───────────────────────────┐   │
│  │        Session Persistence (peekoo.sqlite)        │   │
│  │  ┌─────────────────────────────────────────────┐  │   │
│  │  │ Sessions: runtime_id, provider_id, model   │  │   │
│  │  └─────────────────────────────────────────────┘  │   │
│  └────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────┘
```

## Implementation Batches

### Batch 1: Terminology + Backend Shape

**Goal:** Rename domain model and introduce runtime adapter abstraction

#### Phase 1.1: Rename concepts

**Changes:**
- Replace "provider" with "runtime" for ACP agents
- Keep "provider" for LLM backends only
- Update all DTOs, command names, UI labels

**Files:**
- `crates/peekoo-agent-app/src/agent_provider_service.rs` → `agent_runtime_service.rs`
- `crates/peekoo-agent-app/src/agent_provider_commands.rs` → `agent_runtime_commands.rs`
- Update all type names: `ProviderInfo` → `RuntimeInfo`, etc.
- `apps/desktop-ui/src/types/agent-runtime.ts` (rename from agent-provider.ts)

**Acceptance:**
- No code refers to Codex/OpenCode/Claude Code as "provider"
- All user-facing strings updated

#### Phase 1.2: Define runtime adapter layer

**New module:** `crates/peekoo-agent-app/src/runtime_adapters/`

**Structure:**
```rust
pub trait RuntimeAdapter: Send + Sync {
    fn runtime_id(&self) -> &'static str;
    fn display_name(&self) -> String;
    fn default_command(&self) -> String;
    fn default_args(&self) -> Vec<String>;
    fn install_hint(&self) -> Option<String>;
    fn supported_auth_modes(&self) -> Vec<AuthMode>;
    fn build_launch_env(&self, provider_config: &ProviderConfig) -> HashMap<String, String>;
    fn build_launch_args(&self, base_args: &[String], provider_config: &ProviderConfig, model: &str) -> Vec<String>;
}
```

**Built-in adapters:**
- `CodexAdapter`
- `ClaudeCodeAdapter`
- `OpencodeAdapter`
- `PiAcpAdapter`
- `CustomAdapter` (generic fallback)

**Registry integration:**
- Load known runtime metadata from ACP Registry
- Allow override with local config

**Files:**
- `crates/peekoo-agent-app/src/runtime_adapters/mod.rs`
- `crates/peekoo-agent-app/src/runtime_adapters/codex.rs`
- `crates/peekoo-agent-app/src/runtime_adapters/claude_code.rs`
- `crates/peekoo-agent-app/src/runtime_adapters/opencode.rs`
- `crates/peekoo-agent-app/src/runtime_adapters/pi_acp.rs`
- `crates/peekoo-agent-app/src/runtime_adapters/custom.rs`
- `crates/peekoo-agent-app/src/runtime_adapters/registry.rs`

**Acceptance:**
- Each adapter produces correct launch env/args for its runtime
- Custom adapter uses generic behavior

#### Phase 1.3: Reshape persistence

**Database changes:**

New tables:
```sql
-- ACP runtime configurations
CREATE TABLE agent_runtimes (
    id TEXT PRIMARY KEY,
    runtime_type TEXT NOT NULL, -- codex-acp, claude-code-acp, opencode, custom
    display_name TEXT NOT NULL,
    command TEXT NOT NULL,
    args_json TEXT NOT NULL,
    is_bundled INTEGER NOT NULL DEFAULT 0,
    is_installed INTEGER NOT NULL DEFAULT 0,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'not_installed',
    status_message TEXT,
    install_method TEXT, -- bundled, npx, binary, custom
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- LLM provider configurations per runtime
CREATE TABLE runtime_llm_providers (
    id TEXT PRIMARY KEY,
    runtime_id TEXT NOT NULL REFERENCES agent_runtimes(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL, -- openai, anthropic, etc.
    api_type TEXT NOT NULL,
    base_url TEXT,
    config_json TEXT NOT NULL, -- headers, env vars, etc.
    is_enabled INTEGER NOT NULL DEFAULT 1,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Models per runtime
CREATE TABLE runtime_models (
    id TEXT PRIMARY KEY,
    runtime_id TEXT NOT NULL REFERENCES agent_runtimes(id) ON DELETE CASCADE,
    model_id TEXT NOT NULL,
    display_name TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

-- Update sessions to track runtime/provider/model
ALTER TABLE agent_sessions ADD COLUMN runtime_id TEXT;
ALTER TABLE agent_sessions ADD COLUMN llm_provider_id TEXT;
ALTER TABLE agent_sessions ADD COLUMN model_id TEXT;
```

**Files:**
- `crates/persistence-sqlite/migrations/202603282300_acp_runtime_architecture.sql`
- Update `SessionStore` to use new schema
- Update `SettingsStore` for runtime/provider split

**Acceptance:**
- Migration runs without data loss
- Old sessions remain accessible
- New runtime/provider/model fields populated

#### Phase 1.4: Redesign app-layer API

**New service:** `AgentRuntimeService`

**Methods:**
```rust
pub trait AgentRuntimeService {
    // Runtime management
    async fn list_runtimes(&self) -> Result<Vec<RuntimeInfo>>;
    async fn get_runtime(&self, runtime_id: &str) -> Result<RuntimeInfo>;
    async fn install_runtime(&self, runtime_id: &str, method: InstallMethod) -> Result<InstallResult>;
    async fn uninstall_runtime(&self, runtime_id: &str) -> Result<()>;
    async fn set_default_runtime(&self, runtime_id: &str) -> Result<()>;
    async fn enable_runtime(&self, runtime_id: &str) -> Result<()>;
    async fn disable_runtime(&self, runtime_id: &str) -> Result<()>;
    
    // LLM Provider management (per runtime)
    async fn get_runtime_providers(&self, runtime_id: &str) -> Result<Vec<ProviderInfo>>;
    async fn set_runtime_provider(&self, runtime_id: &str, provider: ProviderConfig) -> Result<()>;
    async fn set_default_provider(&self, runtime_id: &str, provider_id: &str) -> Result<()>;
    
    // Model management (per runtime)
    async fn list_runtime_models(&self, runtime_id: &str) -> Result<Vec<ModelInfo>>;
    async fn add_runtime_model(&self, runtime_id: &str, model: ModelConfig) -> Result<()>;
    async fn remove_runtime_model(&self, runtime_id: &str, model_id: &str) -> Result<()>;
    async fn set_default_model(&self, runtime_id: &str, model_id: &str) -> Result<()>;
    
    // Test connection
    async fn test_runtime(&self, runtime_id: &str) -> Result<TestResult>;
    
    // Launch config for ACP backend
    fn build_launch_config(&self, runtime_id: &str) -> Result<LaunchConfig>;
}
```

**Files:**
- `crates/peekoo-agent-app/src/agent_runtime_service.rs`
- `crates/peekoo-agent-app/src/agent_runtime_commands.rs`
- Update `AgentApplication` to use new service

**Acceptance:**
- All existing provider commands replaced with runtime commands
- Provider commands scoped to runtime
- Launch config includes correct env/args from adapter

#### Phase 1.5: Update Tauri commands

**Commands to add:**
- `list_agent_runtimes()`
- `get_agent_runtime(runtime_id)`
- `install_agent_runtime(runtime_id, method)`
- `uninstall_agent_runtime(runtime_id)`
- `set_default_agent_runtime(runtime_id)`
- `enable_agent_runtime(runtime_id)`
- `disable_agent_runtime(runtime_id)`
- `get_runtime_providers(runtime_id)`
- `set_runtime_provider(runtime_id, config)`
- `set_default_runtime_provider(runtime_id, provider_id)`
- `list_runtime_models(runtime_id)`
- `add_runtime_model(runtime_id, model)`
- `remove_runtime_model(runtime_id, model_id)`
- `set_default_runtime_model(runtime_id, model_id)`
- `test_agent_runtime(runtime_id)`

**Files:**
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- Update command registrations

**Acceptance:**
- All commands return correct runtime/provider/model data
- No references to old mixed-up provider concept

#### Phase 1.6: Tests

**Unit tests:**
- Adapter env/args mapping for each runtime
- Service method behavior
- Database migrations

**Integration tests:**
- Tauri command round-trips
- Launch config generation

**Acceptance:**
- `cargo test --all` passes
- All runtime adapters tested

---

### Batch 2: Settings UI Redesign

**Goal:** Redesign UI around ACP runtime cards with provider/model sub-panels

#### Phase 2.1: Rename UI concepts

**Changes:**
- Rename "Agent Providers" to "ACP Runtimes" or "Agent Runtimes"
- Update all component names, types, labels
- Update navigation

**Files:**
- `apps/desktop-ui/src/features/agent-providers/` → `agent-runtimes/`
- Rename all components
- Update imports

**Acceptance:**
- UI uses "runtime" terminology consistently

#### Phase 2.2: Runtime card component

**Design:**
- Header: Runtime name + active badge + enable toggle
- Section: Command + Arguments (editable)
- Section: Installation status + install button
- Section: Auth configuration (runtime-specific)
- Section: API Provider configuration
- Section: Models (list + add/remove)

**Components:**
- `RuntimeCard.tsx`
- `RuntimeAuthSection.tsx`
- `RuntimeProviderSection.tsx`
- `RuntimeModelsSection.tsx`

**Files:**
- `apps/desktop-ui/src/features/agent-runtimes/RuntimeCard.tsx`
- `apps/desktop-ui/src/features/agent-runtimes/RuntimeAuthSection.tsx`
- `apps/desktop-ui/src/features/agent-runtimes/RuntimeProviderSection.tsx`
- `apps/desktop-ui/src/features/agent-runtimes/RuntimeModelsSection.tsx`

**Acceptance:**
- Matches reference screenshot structure
- All sections functional

#### Phase 2.3: Runtime list page

**Design:**
- Grid of runtime cards
- "Add Custom ACP Runtime" button
- Refresh button
- Filter by status (installed, available, etc.)

**Files:**
- `apps/desktop-ui/src/features/agent-runtimes/AgentRuntimesPanel.tsx`
- `apps/desktop-ui/src/features/agent-runtimes/AddRuntimeDialog.tsx`

**Acceptance:**
- Shows all runtimes
- Custom runtime flow works

#### Phase 2.4: Custom runtime dialog

**Design:**
- Name
- Runtime type selection (or "custom")
- Command
- Arguments
- Working directory
- Initial provider config (optional)

**Files:**
- `apps/desktop-ui/src/features/agent-runtimes/AddRuntimeDialog.tsx`

**Acceptance:**
- Creates custom runtime
- Uses CustomAdapter

#### Phase 2.5: Mount in Settings

**Changes:**
- Add Agent Runtimes section to SettingsPanel
- Remove old Agent Providers section

**Files:**
- `apps/desktop-ui/src/features/settings/SettingsPanel.tsx`

**Acceptance:**
- Runtimes visible in Settings
- Navigation works

---

### Batch 3: Chat Integration

**Goal:** Update chat to use runtime/provider/model correctly

#### Phase 3.1: Update chat header

**Changes:**
- Primary switcher: ACP Runtime
- Secondary controls: Provider + Model (if runtime supports multiple)

**Files:**
- `apps/desktop-ui/src/features/chat/ChatPanel.tsx`
- `apps/desktop-ui/src/features/agent-runtimes/QuickRuntimeSwitcher.tsx` (rename from QuickProviderSwitcher)

**Acceptance:**
- Users switch runtime, not "provider"
- Provider/model shown as subordinate state

#### Phase 3.2: Update chat settings

**Changes:**
- Remove legacy compatible-provider flow
- Show runtime-specific settings
- Show provider/model for active runtime

**Files:**
- `apps/desktop-ui/src/features/chat/settings/ChatSettingsPanel.tsx`

**Acceptance:**
- Settings reflect current runtime
- Provider/model editable per runtime

#### Phase 3.3: Session restore alignment

**Changes:**
- Persist runtime_id, provider_id, model_id with session
- Restore correct tuple on load
- If runtime unavailable, show clear error

**Files:**
- `crates/peekoo-agent-app/src/conversation.rs`
- `crates/peekoo-agent/src/session_store.rs`

**Acceptance:**
- Sessions restore with correct runtime/provider/model
- Clear error when runtime missing

---

### Batch 4: ACP Backend Wiring

**Goal:** Connect adapters to actual ACP process launch

#### Phase 4.1: Launch config integration

**Changes:**
- Update AcpBackend to use runtime adapters
- Generate correct command/args/env

**Files:**
- `crates/peekoo-agent/src/backend/acp.rs`
- `crates/peekoo-agent/src/service.rs`

**Acceptance:**
- Each runtime launches with correct config
- Env vars injected correctly

#### Phase 4.2: Process lifecycle

**Changes:**
- Ensure runtime restart picks up new config
- Handle runtime-specific error states

**Acceptance:**
- Config changes reflect in new sessions
- Errors reported clearly

---

## Testing Strategy

### Unit Tests
- Adapter mapping logic per runtime
- Service method isolation
- Database migrations

### Integration Tests
- Tauri command round-trips
- Full runtime lifecycle
- Session restore scenarios

### E2E Verification
- `cargo test --all`
- `bun run build` in desktop-ui
- Manual UI walkthrough:
  1. View runtimes list
  2. Install codex-acp
  3. Configure provider
  4. Add models
  5. Set default
  6. Chat with correct runtime
  7. Switch runtime in chat
  8. Restore session

## Acceptance Criteria

- [ ] All "provider" references correctly distinguish runtime vs LLM provider
- [ ] Settings page shows ACP Runtimes with cards
- [ ] Each runtime shows: command, args, auth, provider, models
- [ ] Custom runtime creation works
- [ ] Chat uses runtime/provider/model correctly
- [ ] Session restore remembers runtime/provider/model
- [ ] All tests pass
- [ ] No dependency on ACP RFD methods
- [ ] ACP Registry used only for metadata

## Notes

- Keep changes incremental across batches
- Each batch should leave the app in a working state
- Focus on correct terminology first, then features
- Custom runtime is the escape hatch for any unsupported ACP agent