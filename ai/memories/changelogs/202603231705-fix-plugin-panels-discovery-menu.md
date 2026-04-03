## 2026-03-23 17:05: fix: expose discovered plugin panels without runtime reload

**What changed:**
- Updated `list_plugin_panels` to use discovered panel manifests (`all_discovered_ui_panels`) instead of only loaded plugin runtime panels.
- This allows newly installed plugins to appear in the UI plugin menus immediately, without requiring a full app restart.

**Why:**
- Newly installed plugin entries (e.g. Mijia panel) were not visible in `Plugins` menu when plugin runtime had not reloaded yet.

**Files affected:**
- crates/peekoo-agent-app/src/application.rs
- ai/memories/changelogs/202603231705-fix-plugin-panels-discovery-menu.md
