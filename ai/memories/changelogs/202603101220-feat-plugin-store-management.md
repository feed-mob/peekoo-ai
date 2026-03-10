## 2026-03-10 12:20: feat: Add plugin store management improvements

**What changed:**
- Added persisted plugin enable and disable behavior backed by the existing `plugins.enabled` database column.
- Updated startup loading so disabled plugins stay installed but are not loaded into the runtime or exposed as active plugin panels.
- Added Tauri commands and desktop UI controls for enabling and disabling installed plugins.
- Updated the plugin store UI to show update availability and let users trigger updates.
- Hid disabled plugins, and enabled plugins without UI panels, from the sprite plugin submenu.

**Why:**
- Users need a basic plugin catalog plus clear control over whether installed plugins are active in the app.
- Disabled plugins should not appear as runnable UI actions from the sprite menu.

**Files affected:**
- `crates/peekoo-plugin-host/src/registry.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/hooks/use-plugins.ts`
- `apps/desktop-ui/src/features/plugins/PluginManagerPanel.tsx`
- `apps/desktop-ui/src/features/plugins/PluginList.tsx`
- `apps/desktop-ui/src/features/plugins/PluginStoreCatalog.tsx`
- `apps/desktop-ui/src/components/sprite/SpriteActionMenu.tsx`
