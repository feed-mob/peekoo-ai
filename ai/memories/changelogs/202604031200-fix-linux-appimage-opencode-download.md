# Fix: Linux AppImage Build by Removing Bundled OpenCode

**Date:** 2026-04-03

## Summary

Fixed the Linux AppImage build failure caused by shipping multiple ELF binaries in `resources/opencode/`. The npm-based OpenCode bundle contains several platform-specific ELF executables and symlinks that confuse `linuxdeploy` during AppImage assembly.

## Changes

### Release Workflow (`.github/workflows/release.yml`)

- Skip OpenCode staging on Linux (`ubuntu-22.04`) while keeping it for macOS and Windows
- Linux builds now download OpenCode from the ACP registry at first launch instead of bundling

### Tauri Runtime (`apps/desktop-tauri/src-tauri/src/lib.rs`)

**Fallback Path Resolution:**
- Added `resolve_opencode_fallback_path()` that checks:
  1. Previously-installed OpenCode via ACP registry (`~/.peekoo/resources/agents/opencode/`)
  2. System `opencode` on PATH

**Background Installation:**
- Added `spawn_opencode_registry_install()` that uses `AgentApplication::install_registry_agent("opencode", Binary)`
- Download, extraction, and database seeding now all flow through the same ACP registry path as user-initiated installs
- UI notified via `AGENT_SETTINGS_CHANGED_EVENT` when installation completes

**Startup Flow:**
- `AgentState::new()` now uses `or_else` chain: bundled → fallback → None
- Added `needs_opencode_download()` helper to detect when background install is needed
- Download spawns asynchronously after `AgentState` is managed, avoiding circular dependencies

### Dependencies

- Added `which = "7"` to desktop-tauri for PATH lookup
- Removed custom `opencode.rs` module from `peekoo-node-runtime` (unused)
- Reverted `peekoo-node-runtime` changes to original state

## Technical Details

**Root Cause:**
The npm package `opencode-ai` installs multiple platform-specific binaries via `optionalDependencies`:
- `opencode-linux-x64/bin/opencode`
- `opencode-linux-x64-baseline/bin/opencode`
- `opencode-linux-x64-musl/bin/opencode`
- `opencode-linux-x64-baseline-musl/bin/opencode`
- Plus symlinks in `node_modules/.bin/`

`linuxdeploy` scans all ELF files in the AppDir to set rpaths and strip debug symbols. When it encounters self-contained binaries (like these), it attempts dependency analysis and fails.

**Solution:**
Instead of pruning the npm tree, we skip bundling entirely on Linux. OpenCode is downloaded from GitHub releases via the ACP registry at first launch, using the same infrastructure as other registry agents like Gemini or Claude.

## Compatibility

- **macOS/Windows:** No change. Bundled OpenCode still shipped; fallback only activates if bundle missing
- **Linux:** First launch downloads ~50MB from `https://github.com/anomalyco/opencode/releases/`
- **Existing Linux installs:** If OpenCode was previously installed via ACP registry, it's reused without re-download

## Testing

- `cargo clippy` passes on `peekoo-desktop-tauri` and `peekoo-node-runtime`
- `cargo test` passes on `peekoo-node-runtime` (8 tests) and `peekoo-agent-app` (75 tests)

## Related

- Fixes AppImage build failures introduced in release workflow with OpenCode bundling
- Leverages existing ACP registry infrastructure (added in 2026-03-31 ACP registry phase 2)
