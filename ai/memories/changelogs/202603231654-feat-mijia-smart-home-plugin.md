## 2026-03-23 16:54: feat: add mijia smart home plugin with qr login bridge

**What changed:**
- Added a new plugin `mijia-smart-home` with a panel UI for Mijia device management.
- Added a new Tauri command `mijia_plugin_bridge` to execute Python `mijiaAPI` actions from the plugin panel.
- Implemented bridge actions for:
  - auth status check
  - QR login start/finish (persisted auth file)
  - device list with room filtering metadata
  - quick toggle for switch-like properties
  - device detail fetch (properties/actions)
  - property set and action run
- Added `just` recipes to build/install the new plugin.

**Why:**
- Enable first-use QR authentication with local credential persistence and provide practical Mijia device operations directly inside Peekoo plugin panel.

**Files affected:**
- apps/desktop-tauri/src-tauri/src/lib.rs
- apps/desktop-tauri/src-tauri/src/mijia.rs
- plugins/mijia-smart-home/Cargo.toml
- plugins/mijia-smart-home/.cargo/config.toml
- plugins/mijia-smart-home/src/lib.rs
- plugins/mijia-smart-home/peekoo-plugin.toml
- plugins/mijia-smart-home/ui/panel.html
- justfile
- ai/memories/changelogs/202603231654-feat-mijia-smart-home-plugin.md
