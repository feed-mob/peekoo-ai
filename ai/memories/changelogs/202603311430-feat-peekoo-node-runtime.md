## 2026-03-31 14:30: feat: Add peekoo-node-runtime crate for Node.js management

**What changed:**
- Created new `peekoo-node-runtime` crate for managing Node.js runtime
- Ported Zed's node_runtime module to use Tokio instead of smol
- Implemented three Node.js strategies:
  - System Node.js - Uses node/npm from PATH (requires >= v18.0.0)
  - Managed Node.js - Downloads Node.js v20.18.0 LTS to `~/.peekoo/resources/node/`
  - Unavailable fallback - Graceful error handling
- Added NPX package management:
  - Local package installation to per-agent directories
  - Package version checking via package.json parsing
  - NPM command building with proper environment variables
- Archive extraction utilities for tar.gz and zip formats
- HTTP client wrapper using reqwest for downloading Node.js binaries
- Cross-platform support (Linux, macOS, Windows)

**Why:**
- Foundation for supporting NPX-based ACP registry agents (Gemini, Qwen Code, etc.)
- Provides automatic Node.js management without requiring user installation
- Enables installation and execution of Node.js-based AI agents from ACP registry
- Matches Zed's proven architecture adapted for our Tokio-based runtime

**Files created:**
- `crates/peekoo-node-runtime/Cargo.toml` - Crate configuration
- `crates/peekoo-node-runtime/src/lib.rs` - Public API exports
- `crates/peekoo-node-runtime/src/node_runtime.rs` - Core implementation (~650 lines)
- `crates/peekoo-node-runtime/src/http_client.rs` - HTTP client wrapper
- `crates/peekoo-node-runtime/src/archive.rs` - Archive extraction utilities
- `crates/peekoo-node-runtime/src/command.rs` - NPM command builders
- `crates/peekoo-node-runtime/src/paths.rs` - XDG path helpers

**Integration:**
- Added to workspace members in root `Cargo.toml`
- Dependencies: tokio, reqwest, semver, tar, flate2, zip, async-trait, etc.

**Next steps:**
- Create `acp-registry-client` crate to fetch registry from CDN
- Add Tauri commands for registry agent installation
- Update UI to display available ACP registry agents
- Implement binary agent download support (Cursor, Kimi CLI)

**Tests:**
- `cargo test -p peekoo-node-runtime` passes
- `cargo check -p peekoo-node-runtime` succeeds with 1 minor warning
- `cargo fmt -p peekoo-node-runtime` applied
