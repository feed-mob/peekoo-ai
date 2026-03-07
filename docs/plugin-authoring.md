# Plugin Authoring

Peekoo plugins are WASM modules loaded by the `peekoo-plugin-host` crate through Extism.

## Quick Start

1. Install the target:

```bash
rustup target add wasm32-unknown-unknown
```

2. Copy `plugins/example-minimal/`.
3. Update `peekoo-plugin.toml`.
4. Implement your exports in `src/lib.rs`.
5. Build and install:

```bash
just plugin-build example-minimal
just plugin example-minimal
```

## Required Files

```text
my-plugin/
  peekoo-plugin.toml
  Cargo.toml
  .cargo/config.toml
  src/lib.rs
  ui/           # optional
```

## Cargo Setup

```toml
[lib]
crate-type = ["cdylib"]
```

Use `wasm32-unknown-unknown` as the default target.

## Manifest Fields

- `[plugin]` core metadata
- `[permissions]` required and optional capabilities
- `[[tools.definitions]]` agent-callable tools
- `[events]` subscriptions and emitted event names
- `[[data.providers]]` queryable data providers
- `[[ui.panels]]` optional HTML entry points for plugin panels

## Export Conventions

- lifecycle: `plugin_init`
- tools: `tool_{name}`
- data providers: `data_{name}`
- events: `on_event`

## Host Functions

Plugins import host functions with:

```rust
#[host_fn]
extern "ExtismHost" {
    fn peekoo_state_get(input: Json<StateGetRequest>) -> Json<StateGetResponse>;
}
```

Available host functions in v1:

- `peekoo_state_get`
- `peekoo_state_set`
- `peekoo_log`
- `peekoo_emit_event`
- `peekoo_notify`

## Example Plugins

- `plugins/example-minimal/` - smallest copyable template
- `plugins/health-reminders/` - state, events, notifications, tools, data, and UI

## Current Limitations

- plugin panels are currently rendered by the frontend from HTML returned by Tauri
- required permissions are auto-granted during startup install for local development
- plugin tools are surfaced through app/Tauri integration first; deeper `pi_agent_rust` tool registration can be layered in later
