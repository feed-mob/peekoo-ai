# Plan: ACP Registry Integration

## Overview
Implement full support for the ACP (Agent Client Protocol) registry to enable automatic discovery and installation of AI agents. This will replace the current hardcoded runtime list with a dynamic registry system that supports 40+ agents including Gemini, Cursor, Kimi CLI, Qwen Code, and many others.

## Goals
- [ ] Fetch and cache ACP registry from CDN (`https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json`)
- [ ] Support all three distribution types: NPX, Binary, UVX
- [ ] Auto-install agents with proper platform detection
- [ ] Replace "Available Runtimes" UI section with registry data
- [ ] Support 40+ ACP registry agents

## Current State
✅ **peekoo-node-runtime crate** - Node.js management for NPX agents

## Architecture

### New Crate: `acp-registry-client`
```
crates/acp-registry-client/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API
    ├── registry.rs         # Registry fetch & parse
    ├── cache.rs            # Local caching with TTL
    ├── install.rs          # Installation orchestration
    ├── npx.rs              # NPX package installation
    ├── binary.rs           # Binary download & extraction
    └── platform.rs         # Platform detection & matching
```

### Data Structures

```rust
// Registry format (matches official spec)
pub struct Registry {
    pub version: String,
    pub agents: Vec<RegistryAgent>,
}

pub struct RegistryAgent {
    pub id: String,              // "gemini", "cursor", etc.
    pub name: String,            // "Gemini CLI"
    pub version: String,
    pub description: String,
    pub distribution: Distribution,
    pub icon: Option<String>,    // SVG URL
}

pub struct Distribution {
    pub npx: Option<NpxDistribution>,
    pub binary: Option<HashMap<String, BinaryPlatform>>, // platform -> config
    pub uvx: Option<UvxDistribution>,
}
```

## Implementation Steps

### Phase 1: Registry Client Foundation (2-3 days)

**1.1 Create crate structure**
- Add `acp-registry-client` to workspace
- Setup dependencies: reqwest, serde, tokio, semver

**1.2 Registry fetching**
- Implement `RegistryClient::fetch()` 
- Download from CDN with timeout
- Handle offline/network errors gracefully

**1.3 Registry caching**
- Cache to `~/.peekoo/cache/acp-registry.json`
- 1-hour TTL with `cached_at` timestamp
- Icon caching to `~/.peekoo/cache/icons/`

**1.4 Platform detection**
- Detect current OS/arch: `darwin-aarch64`, `linux-x86_64`, etc.
- Filter agents by platform support

### Phase 2: Installation Support (3-4 days)

**2.1 NPX installation** ✅ (leverage peekoo-node-runtime)
```rust
// Uses existing peekoo-node-runtime
runtime.npm_install_packages(
    agent_dir,
    &[(package, version)]
).await?;
```

**2.2 Binary installation**
- Download platform-specific archive
- Extract to `~/.peekoo/resources/agents/<agent-id>/`
- Verify SHA256 checksum (if available)
- Set executable permissions

**2.3 UVX installation** (Future)
- Check if `uv` is installed
- Run `uvx <package> <args>`
- Requires Python ecosystem

**2.4 Installation tracking**
- Store installed agents in SQLite
- Track version, installation method, path
- Support uninstall/reinstall

### Phase 3: Backend Integration (2-3 days)

**3.1 New Tauri commands**
```rust
#[tauri::command]
async fn get_registry_agents() -> Result<Vec<RegistryAgentInfo>>;

#[tauri::command]
async fn install_registry_agent(
    agent_id: String, 
    method: InstallMethod
) -> Result<InstallProgress>;

#[tauri::command]
async fn uninstall_registry_agent(agent_id: String) -> Result<()>;

#[tauri::command]
async fn refresh_registry() -> Result<()>;
```

**3.2 Integration with peekoo-agent-app**
- Add `acp-registry-client` dependency
- Extend `AgentProviderService` to handle registry agents
- Store registry agents in `agent_runtimes` table

**3.3 Migration of existing runtimes**
- Keep hardcoded `opencode`, `pi-acp`, `claude-code`, `codex` as "Featured"
- Merge with registry results for display
- Allow both to coexist

### Phase 4: Frontend Updates (3-4 days)

**4.1 Update Available Runtimes section**
- Replace hardcoded list with registry fetch
- Show loading state during fetch
- Display agent icons from registry
- Show platform compatibility badges

**4.2 Install dialog**
- Show distribution options (NPX/Binary)
- Progress indicator for downloads
- Error handling with retry option

**4.3 Agent management**
- Update "Installed Runtimes" to include registry agents
- Show version and update availability
- Uninstall button with confirmation

### Phase 5: Polish & Testing (2-3 days)

**5.1 Testing**
- Unit tests for registry parsing
- Integration tests for installation
- Mock HTTP server for offline tests

**5.2 Error handling**
- Network failure recovery
- Partial installation cleanup
- Disk space checks

**5.3 Documentation**
- Update AGENTS.md with new architecture
- Add troubleshooting guide

## Files to Modify/Create

### New Files
- `crates/acp-registry-client/Cargo.toml`
- `crates/acp-registry-client/src/lib.rs`
- `crates/acp-registry-client/src/registry.rs`
- `crates/acp-registry-client/src/cache.rs`
- `crates/acp-registry-client/src/install.rs`
- `crates/acp-registry-client/src/npx.rs`
- `crates/acp-registry-client/src/binary.rs`
- `crates/acp-registry-client/src/platform.rs`

### Modified Files
- `Cargo.toml` - Add workspace member
- `crates/peekoo-agent-app/Cargo.toml` - Add dependency
- `crates/peekoo-agent-app/src/agent_provider_service.rs` - Integrate registry
- `apps/desktop-tauri/src-tauri/src/main.rs` - Add Tauri commands
- `apps/desktop-ui/src/features/agent-runtimes/AgentProviderPanel.tsx` - Update UI
- `apps/desktop-ui/src/hooks/useAgentProviders.ts` - Add registry hooks

## Priority Agents to Support First

### Phase 1: NPX agents (easiest)
1. **Gemini CLI** - `@google/gemini-cli`
2. **Qwen Code** - `@qwen-code/qwen-code`
3. **Cline** - `cline`
4. **Auggie** - `@augmentcode/auggie`
5. **GitHub Copilot** - `@github/copilot`

### Phase 2: Binary agents
1. **Cursor** - Download tar.gz/zip from cursor.com
2. **Kimi CLI** - Download from GitHub releases
3. **Goose** - Download from GitHub releases
4. **Codex** - Already have, migrate to binary

### Phase 3: All others
- Full registry support (~40 agents)

## Testing Strategy

### Unit Tests
- Registry JSON parsing
- Platform matching logic
- Cache TTL validation

### Integration Tests
- Download and extract Node.js
- Install NPX package
- Binary download and extraction
- Full install flow

### Manual Testing
- Install each priority agent
- Verify execution works
- Test on macOS/Linux/Windows

## Open Questions

1. **Update strategy**: Check for agent updates on app startup? Manual refresh only?
2. **Binary signing**: Verify code signatures for binary agents?
3. **Version pinning**: Allow users to pin specific versions?
4. **Offline mode**: Cache duration? How to handle registry fetch failure?

## Dependencies

```toml
[dependencies]
# Async
tokio = { version = "1", features = ["fs", "process", "time"] }

# HTTP
reqwest = { version = "0.12", features = ["json", "stream"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Version handling
semver = "1"

# Archive extraction
tar = "0.4"
flate2 = "1"
zip = "2"

# Project crates
peekoo-node-runtime = { path = "../peekoo-node-runtime" }
peekoo-persistence-sqlite = { path = "../persistence-sqlite" }
```

## Timeline Estimate

- **Phase 1** (Registry client): 2-3 days
- **Phase 2** (Installation): 3-4 days  
- **Phase 3** (Backend integration): 2-3 days
- **Phase 4** (Frontend): 3-4 days
- **Phase 5** (Testing/Polish): 2-3 days

**Total: 12-17 days** for full implementation

## Success Criteria

- [ ] Can browse all ACP registry agents in UI
- [ ] Can install NPX agents (Gemini, Qwen) with one click
- [ ] Can install binary agents (Cursor, Kimi) with one click
- [ ] Agents persist across app restarts
- [ ] Can uninstall agents
- [ ] Registry auto-updates every hour
- [ ] Works offline with cached data
