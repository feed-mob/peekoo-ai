## 2026-03-31 16:45: feat: Add acp-registry-client crate for ACP registry integration

**What changed:**
- Created new `acp-registry-client` crate for fetching and installing ACP registry agents
- Implemented full registry data structures matching ACP registry JSON format:
  - `Registry`, `Agent`, `Distribution`, `NpxDistribution`, `BinaryPlatform`, `UvxDistribution`
  - Support for all three distribution methods: NPX, Binary, UVX
- Created `RegistryClient` with automatic caching:
  - Fetches from CDN (`https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json`)
  - 1-hour TTL cache stored at `~/.peekoo/cache/acp-registry.json`
  - Graceful offline fallback (uses cache if fetch fails)
  - Auto-refresh on startup if cache is stale
- Added platform detection and filtering:
  - Supports `darwin-aarch64`, `darwin-x86_64`, `linux-aarch64`, `linux-x86_64`, `windows-x86_64`
  - Filters agents by platform compatibility
  - Preferred method selection (Binary > NPX > UVX)
- Implemented agent installation:
  - NPX: Uses `peekoo-node-runtime` to install npm packages
  - Binary: Downloads and extracts platform-specific archives (tar.gz, zip)
  - UVX: Placeholder for future Python support
  - Installation tracking with `.installed` marker files
- Added agent filtering and sorting:
  - `FilterOptions` with search, platform filtering, method filtering
  - Sort by name, platform support, or featured agents
  - Featured agents list for popular options
- Icon caching support with lazy loading
- File-based cache with proper TTL management
- Comprehensive test coverage:
  - Unit tests for all modules
  - Integration tests with mock HTTP server (wiremock)
  - Cache behavior tests
  - Platform detection tests

**Why:**
- Foundation for supporting all ACP registry agents (40+ agents)
- Replaces hardcoded agent list with dynamic registry discovery
- Enables automatic installation of agents like Gemini, Cursor, Kimi CLI, Qwen Code
- Consistent with Zed's ACP architecture

**Files created:**
- `crates/acp-registry-client/Cargo.toml` - Dependencies and metadata
- `crates/acp-registry-client/src/lib.rs` - Public API exports
- `crates/acp-registry-client/src/types.rs` - Registry data structures
- `crates/acp-registry-client/src/client.rs` - RegistryClient with fetch/cache/refresh
- `crates/acp-registry-client/src/cache.rs` - File-based cache with TTL
- `crates/acp-registry-client/src/platform.rs` - Platform detection & filtering
- `crates/acp-registry-client/src/install.rs` - Installation orchestrator
- `crates/acp-registry-client/src/filter.rs` - Agent filtering & sorting

**Integration:**
- Added to workspace members in root `Cargo.toml`
- Added `peekoo_global_cache_dir()` to `peekoo-paths` for cache location
- Dependencies: reqwest, serde, tokio, chrono, semver, etc.
- 19 tests passing

**Next steps:**
- Integrate with `peekoo-agent-app` (AgentProviderService)
- Add Tauri commands for UI integration
- Update "Available Runtimes" UI to use registry
- Support priority agents: Gemini, Cursor, Kimi CLI, Qwen Code

**Usage Example:**
```rust
use acp_registry_client::{RegistryClient, install, InstallConfig};

// Fetch registry
let client = RegistryClient::new()?;
let registry = client.fetch().await?;

// Filter available agents
let available = filter_agents(&registry.agents, &FilterOptions::default());

// Install an agent
let config = InstallConfig {
    agent: gemini_agent,
    method: Some(InstallMethod::Npx),
    install_dir: PathBuf::from("~/.peekoo/resources/agents/gemini"),
};
let installation = install(config, Some(&node_runtime)).await?;
```
