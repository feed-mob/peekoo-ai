## 2026-03-07 12:00: feat: add WASM plugin system with Extism runtime, plugin manager UI, and example plugins

**What changed:**
- Added `peekoo-plugin-host` crate: Extism-based WASM runtime, plugin registry, manifest parser (`peekoo-plugin.toml`), sandboxed SQLite state/permissions persistence, host functions, event dispatch, and tool bridging.
- Integrated plugin lifecycle management into `peekoo-agent-app`: plugin.rs module for app-layer orchestration, tool injection into agent system prompts, panel HTML assembly.
- Added Tauri command handlers: list/enable/disable/uninstall plugins, fetch panel HTML.
- Built frontend plugin management: PluginList component, PluginManagerPanel, PluginPanelView (sandboxed iframe), PluginsView, use-plugins hook, plugin types, sprite action menu integration with dynamic plugin panel entries.
- Created two example plugins: `example-minimal` (tool-only) and `health-reminders` (tool + UI panel with HTML/CSS/JS).
- Added `docs/plugin-authoring.md` authoring guide.
- Added Justfile recipes: `plugin-build`, `plugin-install`, `plugin`, `plugin-build-all`.
- Updated `.gitignore` to exclude plugin `target/` dirs and `Cargo.lock` files.

**Why:**
- Enable extensibility through a sandboxed plugin architecture so third-party or user-authored plugins can add tools, UI panels, and event hooks without modifying core Peekoo code.
- WASM/Extism provides language-agnostic, sandboxed execution with controlled host function access.
- Local-only plugin store for v1; plugin marketplace deferred to future iteration.

**Code review & cleanup applied:**
- Fixed 5 HIGH priority issues: iframe sandbox hardening (removed `allow-same-origin`), format string injection in tracing, bounds check on plugin inputs, transaction wrapping for state/permissions SQLite ops.
- Fixed 15 MEDIUM priority issues: dead export removal, error propagation improvements (`unwrap` → `ok_or_else`/`map_err`), SRP refactors (extracted `create_agent_service()` helper, moved panel HTML assembly from Tauri into app layer), clippy warning fixes, poisoned mutex logging.
- All builds pass: `just check` (0 warnings), `just test` (75 pass), `just lint` (0 warnings), `bun run build` (clean).

**Deferred (LOW priority):**
- `docs/plugin-authoring.md` missing sections (shutdown, on_event format, data providers, testing).
- `start_tick_timer` in registry.rs is never called (dead code).
- Plugin editions `2021` vs workspace `2024` cosmetic inconsistency.

**Files affected:**
- `Cargo.toml` (workspace member added)
- `Cargo.lock`
- `.gitignore`
- `crates/peekoo-plugin-host/Cargo.toml`
- `crates/peekoo-plugin-host/src/lib.rs`
- `crates/peekoo-plugin-host/src/error.rs`
- `crates/peekoo-plugin-host/src/manifest.rs`
- `crates/peekoo-plugin-host/src/state.rs`
- `crates/peekoo-plugin-host/src/permissions.rs`
- `crates/peekoo-plugin-host/src/events.rs`
- `crates/peekoo-plugin-host/src/runtime.rs`
- `crates/peekoo-plugin-host/src/host_functions.rs`
- `crates/peekoo-plugin-host/src/registry.rs`
- `crates/peekoo-plugin-host/src/tools.rs`
- `crates/peekoo-agent-app/Cargo.toml`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/plugin.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/types/plugin.ts`
- `apps/desktop-ui/src/types/window.ts`
- `apps/desktop-ui/src/hooks/use-plugins.ts`
- `apps/desktop-ui/src/hooks/use-panel-windows.ts`
- `apps/desktop-ui/src/views/PluginPanelView.tsx`
- `apps/desktop-ui/src/views/PluginsView.tsx`
- `apps/desktop-ui/src/views/SpriteView.tsx`
- `apps/desktop-ui/src/features/plugins/PluginList.tsx`
- `apps/desktop-ui/src/features/plugins/PluginManagerPanel.tsx`
- `apps/desktop-ui/src/components/sprite/SpriteActionMenu.tsx`
- `apps/desktop-ui/src/routing/resolve-view.tsx`
- `plugins/example-minimal/` (all files)
- `plugins/health-reminders/` (all files)
- `docs/plugin-authoring.md`
- `docs/TODO.md`
- `justfile`
