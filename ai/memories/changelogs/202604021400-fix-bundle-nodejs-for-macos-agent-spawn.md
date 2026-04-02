---
title: "fix: bundle Node.js in CI to fix macOS agent spawn failure"
date: 2026-04-02
author: opencode
tags: [fix, macos, nodejs, bundling, agent, release]
---

## Summary

Fixed "Failed to spawn ACP agent: No such file or directory (os error 2)" on macOS by bundling a Node.js binary alongside OpenCode in the release build, and fixing the broken PATH injection in `build_launch_env`.

## Problem

- The bundled OpenCode is a shell wrapper that calls `node_modules/.bin/opencode`, which is a JS file with a `#!/usr/bin/env node` shebang
- macOS GUI apps launched from Finder/Dock get a minimal PATH (`/usr/bin:/bin:/usr/sbin:/sbin`) — no Homebrew/nvm `node`
- `build_launch_env` in `runtime_adapters/mod.rs` tried to prepend the managed Node.js path but checked `node_dir().join("bin")` which never exists (actual path is `node_dir()/node-v20.18.0-darwin-arm64/bin/`)
- On Linux this was masked because the user's system `node` is typically on the inherited PATH

## Solution

1. **CI bundling**: Updated `fetch_opencode_bundle.py` to download a Node.js v20.18.0 binary for the target platform and stage it at `resources/opencode/node/`
2. **Wrapper scripts**: Updated Unix/Windows wrapper scripts to prepend the bundled `node` to PATH before exec'ing OpenCode
3. **PATH injection**: Replaced the broken `peekoo_node_runtime::paths::node_dir()` guess in `build_launch_env` with a `node_bin_dir: Option<&Path>` parameter resolved from the Tauri resource bundle
4. **Threading**: Threaded `bundled_node_bin_dir` from `resolve_bundled_node_bin_dir()` in Tauri lib.rs through `AgentApplication` → `AgentProviderService` → all `build_launch_env` call sites

## Files Changed

- `scripts/fetch_opencode_bundle.py` — Added `stage_node_binary()`, Node.js download/extract, updated wrapper scripts
- `apps/desktop-tauri/src-tauri/src/lib.rs` — Added `resolve_bundled_node_bin_dir()`
- `crates/peekoo-agent-app/src/application.rs` — New `bundled_node_bin_dir` field
- `crates/peekoo-agent-app/src/agent_provider_service.rs` — New `bundled_node_bin_dir` field, `node_bin_dir()` accessor
- `crates/peekoo-agent-app/src/runtime_adapters/mod.rs` — Fixed `build_launch_env` signature and PATH logic
- `crates/peekoo-agent-app/src/agent_provider_commands.rs` — Updated call sites

## Testing

- `cargo check` passes
- `cargo test -p peekoo-agent-app` — 75 tests pass
- `cargo check -p peekoo-desktop-tauri` passes
- New test: `build_launch_env_prepends_node_bin_dir_to_path`
