## 2026-03-13 21:30 fix: harden plugin update rollback and registration state

**What changed:**
- Updated `PluginRegistry::install_plugin()` to leave plugins disabled when load/initialize fails instead of persisting an enabled-but-broken state
- Updated `ensure_plugin_row()` to refresh persisted plugin `version` and `manifest_json` for existing rows so store updates no longer leave stale metadata behind
- Extracted store replacement logic in `PluginStoreService` so failed updates reload the restored on-disk plugin into memory when rollback succeeds
- Added regression tests covering failed install disable behavior, manifest metadata refresh, and rollback reload after failed replacement

**Why:**
- Failed plugin updates were restoring files on disk without restoring the plugin runtime, leaving the current session without a previously working plugin
- Failed installs could leave plugins permanently marked enabled even when they could not initialize, causing repeated startup retries
- The `plugins` table needs to reflect the latest manifest metadata after upgrades so the app and store surfaces do not read stale versions or descriptions

**Files affected:**
- `crates/peekoo-plugin-host/src/registry.rs`
- `crates/peekoo-plugin-store/src/lib.rs`
- `ai/memories/changelogs/202603132130-fix-plugin-update-rollback-and-registration-state.md`
