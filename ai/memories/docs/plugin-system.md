# Plugin System

How the WASM-based plugin system works end-to-end — from plugin authoring to runtime execution.

---

## Overview

Peekoo supports extensibility through WASM plugins built with the Extism SDK. Plugins run in a sandboxed environment and can provide agent tools, UI panels, and event hooks. The plugin system uses a local-only store (no marketplace in v1).

---

## Architecture

```
Plugin Author                    Peekoo Runtime
─────────────                    ──────────────
Rust code (PDK)                  
  ↓ cargo build                  
  ↓ --target wasm32-unknown-unknown
  ↓                              
plugin.wasm + peekoo-plugin.toml 
  ↓ just plugin-install          
  ↓                              
~/.peekoo/plugins/<name>/        PluginRegistry::scan_and_load()
                                   ↓
                                 PluginRuntime (Extism)
                                   ↓
                                 Host functions ↔ Plugin calls
                                   ↓
                                 Tools → Agent system prompt
                                 Panels → Tauri webview (iframe)
                                 Events → on_event dispatch
```

### Dependency Flow

```
desktop-ui → desktop-tauri → peekoo-agent-app → peekoo-plugin-host
                                                   ├── manifest.rs   (TOML parser)
                                                   ├── registry.rs   (scan, load, lifecycle)
                                                   ├── runtime.rs    (Extism WASM runtime)
                                                   ├── host_functions.rs (log, state, http)
                                                   ├── tools.rs      (tool call bridging)
                                                   ├── events.rs     (event dispatch)
                                                   ├── state.rs      (SQLite key-value)
                                                   ├── permissions.rs (SQLite grants)
                                                   └── error.rs      (PluginError)
```

---

## Plugin Manifest (`peekoo-plugin.toml`)

Every plugin directory must contain a `peekoo-plugin.toml`:

```toml
[plugin]
name = "my-plugin"
version = "0.1.0"
description = "What this plugin does"
author = "Your Name"
wasm = "target/wasm32-unknown-unknown/release/my_plugin.wasm"

[[tools]]
name = "tool_name"
description = "What the tool does"

[[tools.parameters]]
name = "param_name"
type = "string"
description = "Parameter description"
required = true

[ui]
panel = "ui/panel.html"
panel_label = "My Panel"
```

### Sections

| Section | Purpose |
|---------|---------|
| `[plugin]` | Name, version, description, author, WASM binary path |
| `[[tools]]` | Tool definitions exposed to the agent |
| `[ui]` | Optional panel HTML path and label |
| `[permissions]` | Requested permissions (network, filesystem, etc.) |

---

## Plugin Directories

Plugins are discovered from two locations:

1. **Global:** `~/.peekoo/plugins/` — user-installed plugins
2. **Workspace-local:** `plugins/` — project-specific plugins (walked up from CWD)

Each plugin lives in its own subdirectory containing `peekoo-plugin.toml` and the compiled `.wasm` file.

---

## Host Functions

Plugins call host functions via the Extism PDK:

| Host Function | Purpose |
|---------------|---------|
| `host_log` | Write structured log entry (level + message) |
| `host_state_get` | Read from sandboxed key-value store |
| `host_state_set` | Write to sandboxed key-value store |
| `host_http_request` | Make HTTP request (subject to permissions) |

All host functions are scoped per-plugin (state keys are namespaced by plugin name).

---

## Plugin Exports (Guest Functions)

Plugins must export these functions:

| Export | When Called | Input | Output |
|--------|------------|-------|--------|
| `on_load` | Plugin initialization | `{}` | Status JSON |
| `call_tool` | Agent invokes a plugin tool | `{ "name": "...", "arguments": {...} }` | Result string |
| `on_event` | System event dispatched | Event JSON | `{}` |

---

## Tool Integration

Plugin tools are surfaced to the agent by appending capability descriptions to the system prompt. When the agent decides to use a plugin tool:

1. Agent output includes tool call with plugin tool name
2. `peekoo-agent-app` routes the call to `PluginRegistry::call_tool()`
3. Registry finds the owning plugin and calls `call_tool` on the WASM instance
4. Result is returned to the agent as tool output

---

## UI Panel Integration

Plugins with a `[ui]` section get a panel in the sprite action menu:

1. `SpriteActionMenu` queries enabled plugins and renders entries for those with panels
2. Clicking opens a Tauri webview window routed to `PluginPanelView`
3. `PluginPanelView` fetches assembled HTML via `plugin_panel_html` Tauri command
4. HTML is rendered in a sandboxed `<iframe>` (`sandbox="allow-scripts"`)
5. Panel HTML/CSS/JS are inlined by `AgentApplication::plugin_panel_html()`

---

## State & Permissions (SQLite)

Plugin state and permissions are stored in the app's SQLite database:

### `plugin_state` table
```sql
CREATE TABLE plugin_state (
    plugin_name TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL
);
```

### `plugin_permissions` table
```sql
CREATE TABLE plugin_permissions (
    plugin_name TEXT NOT NULL,
    permission TEXT NOT NULL,
    granted INTEGER NOT NULL DEFAULT 0
);
```

Both use DELETE+INSERT wrapped in transactions (no UNIQUE constraints in schema).

---

## Plugin Lifecycle

```
scan_and_load()
  ↓
Parse peekoo-plugin.toml → PluginManifest
  ↓
Create Extism Plugin (WASI enabled)
  ↓
Call on_load() → plugin initializes
  ↓
Register tools, panels, event subscriptions
  ↓
Plugin is "enabled" and active
  ↓
... runtime: call_tool(), on_event(), state ops ...
  ↓
disable/uninstall → remove from registry
```

---

## Key Files

| File | Responsibility |
|------|----------------|
| `crates/peekoo-plugin-host/src/manifest.rs` | Parse `peekoo-plugin.toml` into `PluginManifest` |
| `crates/peekoo-plugin-host/src/registry.rs` | Scan dirs, load/enable/disable/uninstall plugins |
| `crates/peekoo-plugin-host/src/runtime.rs` | Create Extism `Plugin` instances with WASI |
| `crates/peekoo-plugin-host/src/host_functions.rs` | Host function implementations |
| `crates/peekoo-plugin-host/src/tools.rs` | Route tool calls to correct plugin |
| `crates/peekoo-plugin-host/src/events.rs` | Dispatch events to subscribed plugins |
| `crates/peekoo-plugin-host/src/state.rs` | SQLite key-value state per plugin |
| `crates/peekoo-plugin-host/src/permissions.rs` | SQLite permission grants per plugin |
| `crates/peekoo-agent-app/src/plugin.rs` | App-layer plugin orchestration |
| `crates/peekoo-agent-app/src/application.rs` | Plugin init, tool injection, panel HTML assembly |
| `apps/desktop-tauri/src-tauri/src/lib.rs` | Tauri commands for plugin management |
| `apps/desktop-ui/src/hooks/use-plugins.ts` | React hook for plugin list queries |
| `apps/desktop-ui/src/features/plugins/PluginList.tsx` | Plugin manager list UI |
| `apps/desktop-ui/src/views/PluginPanelView.tsx` | Sandboxed iframe for plugin panels |
| `apps/desktop-ui/src/types/plugin.ts` | TypeScript types and Zod schemas |
| `docs/plugin-authoring.md` | Plugin authoring guide |

---

## Example Plugins

### example-minimal
Tool-only plugin demonstrating the minimal PDK setup. Exports `greet` and `echo` tools.
Location: `plugins/example-minimal/`

### health-reminders
Full-featured plugin with tools (`get_health_tip`, `set_reminder_interval`, `get_reminder_status`) and a UI panel (HTML/CSS/JS) showing health tips with a timer.
Location: `plugins/health-reminders/`

---

## Building & Installing Plugins

```bash
# Build a single plugin
just plugin-build example-minimal

# Install to ~/.peekoo/plugins/
just plugin-install example-minimal

# Build + install in one step
just plugin example-minimal

# Build all plugins
just plugin-build-all
```

---

## Security Notes

- WASM runs in Extism sandbox — no direct filesystem/network access
- Host functions mediate all I/O (state, HTTP, logging)
- UI panels render in `sandbox="allow-scripts"` iframe (no `allow-same-origin`)
- Permissions model exists but v1 grants all by default
- Plugin state is namespaced by plugin name in SQLite
