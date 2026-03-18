## 2026-03-18 17:30 fix: Stabilize OpenClaw Sessions plugin windowing and gateway access

**What changed:**
- Restored the shared `PanelShell` wrapper for plugin panels so the OpenClaw Sessions window regains the standard drag region, title rendering, and close button
- Removed the plugin-panel-specific close interception from the iframe bridge so plugin commands flow through normal Tauri invoke handling
- Added websocket allowlist support for `*` in the plugin host and switched the OpenClaw Sessions manifest to use it
- Reset the OpenClaw default websocket URL back to `ws://127.0.0.1:18789` across the plugin manifest, AssemblyScript runtime, and panel UI defaults
- Fixed the OpenClaw `sessions.list` call to stop sending unsupported `page` and `pageSize` fields to the gateway
- Kept pagination local to the panel UI and surfaced `defaultPageSize` from plugin config so the plugin setting actually controls the panel page size
- Rebuilt the OpenClaw Sessions WASM artifact after each AssemblyScript change

**Why:**
- Plugin panels were being opened as undecorated windows without the shared shell, which made them difficult to drag and sometimes impossible to close
- The host-level websocket allowlist was blocking saved OpenClaw URLs that did not match the manifest port restriction
- The OpenClaw gateway rejected the plugin’s `sessions.list` request because `page` and `pageSize` are not valid RPC parameters
- The `default_page_size` plugin config existed in the manifest but had no effect until the panel started reading it

**Files affected:**
- `apps/desktop-ui/src/views/PluginPanelView.tsx`
- `apps/desktop-ui/src/components/panels/PanelShell.tsx`
- `crates/peekoo-plugin-host/src/host_functions.rs`
- `plugins/openclaw-sessions/peekoo-plugin.toml`
- `plugins/openclaw-sessions/assembly/index.ts`
- `plugins/openclaw-sessions/ui/panel.html`
- `plugins/openclaw-sessions/build/openclaw_sessions.wasm`
