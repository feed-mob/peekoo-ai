## 2026-03-18 00:00 feat: global settings panel with animated sprite selector

**What changed:**
- Added new `peekoo-app-settings` crate with `AppSettingsStore` (key-value SQLite CRUD) and `AppSettingsService` (sprite-aware facade with validation); 8 unit tests
- Added SQLite migration `0004_global_settings.sql` creating the `app_settings` key-value table (key, value, updated_at)
- Wired `AppSettingsService` into `AgentApplication` in `peekoo-agent-app` alongside existing services, sharing the same SQLite connection
- Added three Tauri commands: `app_settings_get`, `app_settings_set`, `app_settings_list_sprites`
- Added a "Settings" item to the system tray menu (between Show/Hide Pet and Quit Peekoo) that emits an `open-settings` event to the frontend
- Added `panel-settings` window (420×500) to the window registry and router
- Created `SettingsView`, `SettingsPanel`, `useGlobalSettings` hook, and `SpriteSelector` component
- `SpriteSelector` renders a 2-column grid of sprite cards; each card shows a live `SpriteAnimation` preview playing the idle animation at the sprite's native manifest scale, with a checkmark on the active selection
- `Sprite.tsx` now loads the active sprite ID from `app_settings_get` on mount and reacts to `sprite:changed` events; default remains `dark-cat`
- `SpriteView` listens for the `open-settings` tray event and opens/focuses the settings panel

**Why:**
- The active sprite was hardcoded to `"dark-cat"` with no way for users to switch pets
- A dedicated global settings crate keeps app-level user preferences (sprite, future: language/theme) separate from agent concerns in `peekoo-agent-app`, following SRP
- The key-value table design makes the settings layer trivially extensible for future preferences without schema migrations
- Showing live animated previews in the selector gives users an accurate preview of how their chosen pet will actually look on the desktop

**Files affected:**
- `crates/peekoo-app-settings/Cargo.toml` (new)
- `crates/peekoo-app-settings/src/lib.rs` (new)
- `crates/peekoo-app-settings/src/dto.rs` (new)
- `crates/peekoo-app-settings/src/store.rs` (new)
- `crates/peekoo-app-settings/src/service.rs` (new)
- `crates/persistence-sqlite/migrations/0004_global_settings.sql` (new)
- `crates/persistence-sqlite/src/lib.rs`
- `Cargo.toml`
- `crates/peekoo-agent-app/Cargo.toml`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/types/global-settings.ts` (new)
- `apps/desktop-ui/src/types/window.ts`
- `apps/desktop-ui/src/routing/resolve-view.tsx`
- `apps/desktop-ui/src/views/SettingsView.tsx` (new)
- `apps/desktop-ui/src/features/settings/SettingsPanel.tsx` (new)
- `apps/desktop-ui/src/features/settings/useGlobalSettings.ts` (new)
- `apps/desktop-ui/src/features/settings/SpriteSelector.tsx` (new)
- `apps/desktop-ui/src/components/sprite/Sprite.tsx`
- `apps/desktop-ui/src/hooks/use-panel-windows.ts`
- `apps/desktop-ui/src/views/SpriteView.tsx`
