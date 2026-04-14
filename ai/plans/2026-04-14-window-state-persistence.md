# Plan: Tauri Window State Persistence

## Overview
Persist and restore window position/size for the sprite main window and every `panel-*` window by integrating Tauri v2's native `window-state` plugin.

## Goals
- [x] Use native Tauri v2 window-state persistence instead of custom storage logic.
- [x] Cover both the main sprite window and dynamically created panel windows.
- [x] Keep existing panel labels and creation flow unchanged.
- [x] Ensure capability permissions allow window-state plugin commands.

## Design

### Approach
Use the official `tauri-plugin-window-state` plugin in the Rust runtime. This plugin automatically saves and restores window bounds keyed by window label across app launches.

### Components
- `apps/desktop-tauri/src-tauri/Cargo.toml`: add Rust plugin dependency.
- `apps/desktop-tauri/src-tauri/src/lib.rs`: register plugin in Tauri builder.
- `apps/desktop-tauri/src-tauri/capabilities/default.json`: grant `window-state:default` permission.

## Implementation Steps

1. **Add plugin dependency**
   - Add `tauri-plugin-window-state` to desktop-tauri crate dependencies.

2. **Register plugin in runtime**
   - Initialize `tauri_plugin_window_state::Builder::default().build()` in the existing plugin chain.

3. **Enable permissions**
   - Add `window-state:default` to default capability permissions used by `main` and `panel-*` windows.

4. **Verify behavior**
   - Run Rust compile checks.
   - Manually verify that moving/resizing windows is restored after app restart.

## Files to Modify/Create
- `ai/plans/2026-04-14-window-state-persistence.md` - plan record.
- `apps/desktop-tauri/src-tauri/Cargo.toml` - add dependency.
- `apps/desktop-tauri/src-tauri/src/lib.rs` - register plugin.
- `apps/desktop-tauri/src-tauri/capabilities/default.json` - add permission.

## Testing Strategy
- Compile-time validation via `cargo check -p peekoo-desktop-tauri`.
- Manual validation:
  - Move `main` and each opened `panel-*` window.
  - Restart app and confirm each window reopens at last position.

## Open Questions
- None for initial integration.
