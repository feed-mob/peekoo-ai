## 2026-03-09 18:30: feat: add plugin store with GitHub catalog, install/uninstall

**What changed:**
- Added `peekoo-plugin-store` crate: `PluginStoreService` (fetch_catalog, install_plugin, uninstall_plugin), `StorePluginDto` with `PluginSource` enum, GitHub API integration (contents API for listing, raw URLs for downloads, recursive directory download).
- Wired store service into `AgentApplication` with 3 new methods (`store_catalog`, `store_install`, `store_uninstall`).
- Added 3 Tauri commands: `plugin_store_catalog`, `plugin_store_install`, `plugin_store_uninstall`.
- Built frontend store UI: `PluginStoreCatalog` component, `usePluginStore` hook with catalog fetch/install/uninstall/per-plugin loading state, tab navigation in `PluginManagerPanel` (Installed/Store), Remove button on installed plugins.
- Updated `.gitignore` to allow WASM binaries in `plugins/*/target/wasm32-unknown-unknown/release/*.wasm` while excluding other build artifacts.
- Built and staged WASM binaries for both example plugins.

**Why:**
- Enable users to discover and install plugins from the GitHub repository at runtime without manual file management.
- Plugins install to `~/.peekoo/plugins/<key>/` (global data dir), separate from workspace development.

**Code review & fixes applied:**
- CRITICAL: Fixed `.gitignore` to properly exclude 655 non-wasm build artifacts (corrected negation pattern with intermediate directory un-ignore).
- UX: Store tab now shows Remove button for all installed plugins regardless of source.
- UX: Plugin icon in sprite menu always opens sub-menu (was previously bypassing when no plugin panels existed).
- MEDIUM: Moved `rusqlite` from `[dependencies]` to `[dev-dependencies]` in peekoo-plugin-store.
- MEDIUM: Added cleanup on partial download failure (removes dest_dir on error).
- LOW: Replaced stringly-typed `"store"/"workspace"/"none"` with `PluginSource` enum.
- LOW: Wrapped `isInstalling` in `useCallback` for stable identity.
- LOW: Added recursion depth limit (10) on `download_directory_recursive`.
- LOW: Added fetch dedup guard in `fetchCatalog` to prevent concurrent requests from rapid tab switching.

**Deferred:**
- Blocking I/O on main thread for store operations (matches existing Tauri command pattern, would require architectural change).

**Files affected:**
- `Cargo.toml` (workspace member added)
- `Cargo.lock`
- `.gitignore`
- `crates/peekoo-plugin-store/Cargo.toml`
- `crates/peekoo-plugin-store/src/lib.rs`
- `crates/peekoo-agent-app/Cargo.toml`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/types/plugin.ts`
- `apps/desktop-ui/src/hooks/use-plugin-store.ts`
- `apps/desktop-ui/src/features/plugins/PluginStoreCatalog.tsx`
- `apps/desktop-ui/src/features/plugins/PluginManagerPanel.tsx`
- `apps/desktop-ui/src/features/plugins/PluginList.tsx`
- `apps/desktop-ui/src/components/sprite/SpriteActionMenu.tsx`
- `plugins/example-minimal/target/wasm32-unknown-unknown/release/example_minimal.wasm`
- `plugins/health-reminders/target/wasm32-unknown-unknown/release/health_reminders.wasm`
