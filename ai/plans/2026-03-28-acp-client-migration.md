# ACP Client Migration Plan

**Date:** 2026-03-28  
**Status:** In Progress  
**Goal:** Replace `pi_agent_rust` with an ACP (Agent Client Protocol) client architecture

## Overview

Replace the embedded `pi_agent_rust` library with an ACP client architecture where `peekoo-agent` spawns and connects to external agent harnesses (pi-acp, opencode, claude-code, codex). Peekoo maintains its own session persistence in SQLite.

## Key Requirements

1. **Primary Provider**: pi-acp (bundled, works out of box)
2. **Built-in Support**: opencode, claude-code, codex (auto-download or use npx)
3. **Custom Providers**: Support custom ACP agents via configuration
4. **Session Persistence**: Peekoo-managed in SQLite
5. **Provider Switching**: Support mid-conversation switching
6. **Tool Execution**: Through peekoo's MCP server
7. **Authentication**: Each agent manages its own credentials

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    peekoo-agent (ACP Client)                     │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              Backend Trait (AgentBackend)                  │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │  │
│  │  │   AcpBackend │  │ AcpBackend   │  │ AcpBackend   │     │  │
│  │  │   (pi-acp)   │  │ (opencode)   │  │(claude-code)│     │  │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │  │
│  │         │                  │                  │             │  │
│  │         └──────────────────┼──────────────────┘             │  │
│  │                            │ ACP Protocol                   │  │
│  │                   ┌──────────▼──────────┐                    │  │
│  │                   │  ACP Client Core    │                    │  │
│  │                   │ (spawn, communicate)│                    │  │
│  │                   └──────────┬──────────┘                    │  │
│  └──────────────────────────────┼───────────────────────────────┘  │
│                                 │                                   │
│  ┌──────────────────────────────▼───────────────────────────────┐  │
│  │              Session Manager (peekoo-managed)               │  │
│  │  ┌──────────────────┐      ┌──────────────────┐              │  │
│  │  │ agent_sessions   │      │ session_messages │              │  │
│  │  │   (metadata)     │◄────►│   (conversation) │              │  │
│  │  └──────────────────┘      └──────────────────┘              │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │              MCP Tool Registry (peekoo-managed)                │  │
│  │  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐          │  │
│  │  │  peekoo     │  │  Plugin      │  │  mcporter    │          │  │
│  │  │  MCP Server │  │  Tools       │  │  (fallback)  │          │  │
│  │  └─────────────┘  └──────────────┘  └──────────────┘          │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 0: Database Schema ✅ COMPLETE

**Migration 1:** `202603281200_agent_session_storage.sql`
- `agent_sessions`: Session metadata with provider info
- `session_messages`: Conversation history with provider tracking
- `session_tool_results`: Tool execution cache

**Migration 2:** `202603281300_agent_provider_configs.sql`
- `agent_providers`: Provider installation and configuration
- `agent_provider_installations`: Installation tracking
- `agent_session_providers`: Provider switch history

### Phase 1: Backend Trait & ACP Client (IN PROGRESS)

**Files to Create:**
1. `crates/peekoo-agent/src/backend/mod.rs` - AgentBackend trait
2. `crates/peekoo-agent/src/backend/acp.rs` - ACP client implementation
3. Tests for backend initialization and communication

**AgentBackend Trait:**
- `initialize()` - Initialize with config
- `prompt()` - Send prompt with conversation history
- `set_model()` - Switch model/provider
- `cancel()` - Cancel in-flight prompt
- `provider_state()` - Get opaque provider state for persistence
- `restore_provider_state()` - Restore provider state

### Phase 2: Session Persistence

**Files to Create:**
1. `crates/peekoo-agent/src/session_store.rs` - SQLite persistence layer

**Features:**
- Create/resume sessions
- Load/save conversation history
- Track provider switches per session
- Store provider-specific opaque state

### Phase 3: MCP Tool Bridge

**Files to Create:**
1. `crates/peekoo-agent/src/mcp_bridge.rs` - MCP tool execution bridge

**Features:**
- Connect to peekoo's MCP server
- Execute tools on behalf of ACP agents
- Cache tool results

### Phase 4: Refactor AgentService

**Files to Modify:**
1. `crates/peekoo-agent/src/service.rs` - Use AgentBackend trait
2. `crates/peekoo-agent/src/config.rs` - Add provider configuration

**Features:**
- Replace AgentSessionHandle with AgentBackend
- Support provider switching at runtime
- Maintain existing public API

### Phase 5: Provider Management Service

**Files to Create:**
1. `crates/peekoo-agent-app/src/agent_provider_service.rs`

**Tauri Commands:**
- `list_agent_providers()`
- `install_agent_provider()`
- `set_default_provider()`
- `get_provider_config()`
- `update_provider_config()`
- `test_provider_connection()`
- `add_custom_provider()`

### Phase 6: UI Components

**Files to Create:**
1. `apps/desktop-ui/src/views/AgentProvidersView.tsx`
2. `apps/desktop-ui/src/features/agent-providers/AgentProviderPanel.tsx`
3. `apps/desktop-ui/src/features/agent-providers/ProviderCard.tsx`
4. `apps/desktop-ui/src/features/agent-providers/InstallProviderDialog.tsx`
5. `apps/desktop-ui/src/features/agent-providers/QuickProviderSwitcher.tsx`
6. `apps/desktop-ui/src/hooks/useAgentProviders.ts`
7. `apps/desktop-ui/src/types/agent-provider.ts`

### Phase 7: Cleanup

**Tasks:**
- Remove `pi` dependency from Cargo.toml files
- Update documentation
- Migration guide for users

## Installation Methods

### Priority Order:
1. **Bundled** (pi-acp only): Auto-available, no setup needed
2. **npx**: Check for Node.js, auto-install if available
3. **Binary download**: Download pre-built binaries for platform
4. **Custom**: User specifies path or command

### Platform Support:
- Linux (x86_64, ARM64)
- macOS (x86_64, ARM64)
- Windows (x64)

## UI/UX Design

### Settings Page: Agent Providers
- Grid of provider cards showing status
- Install/Configure/Remove actions
- Default provider indicator
- Add custom provider button

### Installation Dialog:
- Choose installation method (bundled/npx/binary/custom)
- Show prerequisites check (Node.js availability)
- Download progress for binaries
- Post-install configuration

### Quick Provider Switcher (Chat Panel):
- Dropdown in chat header
- Shows current provider and model
- Switch with confirmation
- Preserve conversation history warning

## Database Schema Details

### agent_sessions
```sql
id TEXT PRIMARY KEY
title TEXT
status TEXT -- active, paused, closed
current_provider TEXT -- pi-acp, opencode, etc.
provider_command TEXT
provider_args_json TEXT
working_directory TEXT
persona_dir TEXT
system_prompt TEXT
skills_json TEXT
provider_state_json TEXT -- opaque provider-specific state
created_at TEXT
updated_at TEXT
last_activity_at TEXT
closed_at TEXT
```

### session_messages
```sql
id TEXT PRIMARY KEY
session_id TEXT FK
sequence_num INTEGER
role TEXT -- system, user, assistant, tool
content_type TEXT -- text, tool_call, tool_result, thinking
content_json TEXT
tool_name TEXT
tool_call_id TEXT
provider TEXT -- which provider generated this
model TEXT
input_tokens INTEGER
output_tokens INTEGER
created_at TEXT
```

### agent_providers
```sql
id TEXT PRIMARY KEY
provider_id TEXT UNIQUE -- 'pi-acp', 'opencode', etc.
display_name TEXT
description TEXT
is_bundled INTEGER
installation_method TEXT -- bundled, npx, binary, custom
command TEXT
args_json TEXT
binary_path TEXT
download_url TEXT
checksum TEXT
is_installed INTEGER
is_default INTEGER
status TEXT -- not_installed, installing, ready, error
status_message TEXT
installed_at TEXT
updated_at TEXT
config_json TEXT
env_vars_json TEXT
```

## Configuration

### Environment Variables:
- `PEEKOO_AGENT_PROVIDER` - Default provider ID
- `PEEKOO_AGENT_PROVIDER_PATH` - Custom provider binary path
- `PEEKOO_AGENT_SESSION_DIR` - Session storage directory

### Project Config (`.peekoo/config.toml`):
```toml
[agent]
default_provider = "pi-acp"
session_dir = ".peekoo/sessions"

[agent.providers.pi-acp]
enabled = true
default_model = "claude-3.5-sonnet"

[agent.providers.opencode]
enabled = true
installation = "npx"  # or "binary"
```

## Testing Strategy

### Unit Tests:
- Backend trait implementations
- Session store CRUD operations
- MCP bridge tool execution
- Provider configuration parsing

### Integration Tests:
- ACP protocol communication
- Session persistence round-trip
- Provider switching
- Tool execution with real MCP server

### End-to-End Tests:
- Full chat session with provider switching
- Installation flow
- UI interactions

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| pi-acp not available as binary | Build from source, bundle with app |
| Provider switching loses context | Full history reload, system message notification |
| Tool execution failures | Comprehensive error handling, fallback to mcporter |
| Session migration complexity | New schema, no backward compatibility needed |
| Credential management | Let agents manage their own auth |

## Success Criteria

- [x] Phase 0: SQLite migrations created and passing tests
- [ ] Phase 1: AgentBackend trait and AcpBackend implemented
- [ ] Phase 2: SessionStore with full persistence
- [ ] Phase 3: MCP bridge working
- [ ] Phase 4: AgentService refactored, all tests passing
- [ ] Phase 5: Provider management service and Tauri commands
- [ ] Phase 6: UI components for provider management
- [ ] Phase 7: pi_agent_rust dependency removed

## Notes

- **mcporter**: For agents that don't support MCP natively
- **Session portability**: Can resume with different providers
- **Provider state**: Opaque JSON stored by peekoo, used by agent
- **Always use latest**: For npx-based providers, always use latest version
