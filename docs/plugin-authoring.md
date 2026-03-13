# Plugin Authoring

Peekoo plugins are WASM modules loaded by the `peekoo-plugin-host` crate through [Extism](https://extism.org/). Plugins can be written in **Rust** or **AssemblyScript** (TypeScript).

## Quick Start (Rust)

1. Install the WASM target:

```bash
rustup target add wasm32-wasip1
```

2. Scaffold a new plugin from the template:

```bash
cargo generate --path plugins/template-rust --destination plugins --name my-plugin
```

Or copy `plugins/example-minimal/` into `plugins/my-plugin/` and rename.

3. Update `peekoo-plugin.toml` with your plugin metadata.

4. Implement your exports in `src/lib.rs`.

5. Build and install:

```bash
just plugin-build my-plugin
just plugin-install my-plugin
```

### Minimal Rust Example

```rust
#![no_main]
use peekoo_plugin_sdk::prelude::*;

#[derive(Deserialize)]
struct EchoInput { input: String }

#[derive(Serialize)]
struct EchoOutput { echo: String, call_count: u64 }

#[plugin_fn]
pub fn plugin_init(_: String) -> FnResult<String> {
    peekoo::log::info("plugin started");
    Ok(r#"{"status":"ok"}"#.into())
}

#[plugin_fn]
pub fn tool_my_echo(Json(req): Json<EchoInput>) -> FnResult<Json<EchoOutput>> {
    let count: u64 = peekoo::state::get("call_count")?.unwrap_or(0);
    peekoo::state::set("call_count", &(count + 1))?;
    Ok(Json(EchoOutput { echo: req.input, call_count: count + 1 }))
}
```

## Quick Start (AssemblyScript)

1. Copy `plugins/as-example-minimal/`.

2. Install dependencies:

```bash
cd plugins/my-as-plugin
bun install
```

3. Update `peekoo-plugin.toml` with your plugin metadata.

4. Implement your exports in `assembly/index.ts`.

5. Build and install:

```bash
just plugin-build-as my-as-plugin
just plugin-install-as my-as-plugin
```

### Minimal AssemblyScript Example

```typescript
import { Host } from "@extism/as-pdk";
import * as state from "@peekoo/plugin-sdk/assembly/state";
import * as log from "@peekoo/plugin-sdk/assembly/log";

export function plugin_init(): i32 {
    log.info("plugin started");
    Host.outputString('{"status":"ok"}');
    return 0;
}

export function tool_my_echo(): i32 {
    const input = Host.inputString();
    // parse input, do work, write output
    Host.outputString('{"echo":"hello"}');
    return 0;
}
```

## Required Files

### Rust Plugin

```text
my-plugin/
  Cargo.toml           # [workspace], cdylib, peekoo-plugin-sdk dep
  .cargo/config.toml   # target = "wasm32-wasip1"
  peekoo-plugin.toml   # plugin manifest
  src/lib.rs           # plugin code
  ui/                  # optional panel HTML/JS/CSS
```

### AssemblyScript Plugin

```text
my-plugin/
  package.json         # @peekoo/plugin-sdk + @extism/as-pdk deps
  asconfig.json        # asc compiler config
  tsconfig.json        # AS type definitions
  peekoo-plugin.toml   # plugin manifest
  assembly/
    index.ts           # plugin code
  ui/                  # optional panel HTML/JS/CSS
```

## Cargo Setup (Rust)

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[workspace]

[lib]
crate-type = ["cdylib"]

[dependencies]
peekoo-plugin-sdk = { path = "../../crates/peekoo-plugin-sdk" }
extism-pdk = "1.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

The generated project must live under `plugins/<name>/` for the relative SDK path and `just plugin-*` recipes to work. The `.cargo/config.toml` must set `target = "wasm32-wasip1"`.

## Plugin Manifest (`peekoo-plugin.toml`)

### Required fields

```toml
[plugin]
key = "my-plugin"              # unique identifier (kebab-case)
name = "My Plugin"             # display name
version = "0.1.0"
author = "Your Name"
description = "What this plugin does"
min_peekoo_version = "0.1.0"
wasm = "target/wasm32-wasip1/release/my_plugin.wasm"  # path to built WASM
```

### Permissions

```toml
[permissions]
required = ["state:read", "state:write"]
optional = ["agent:register-tool"]
```

### Tool definitions

```toml
[[tools.definitions]]
name = "my_tool"
description = "What this tool does"
parameters = '{"type":"object","properties":{"input":{"type":"string"}},"required":["input"]}'
return_type = "object"
```

### Event subscriptions

```toml
[events]
subscribe = ["schedule:fired", "system:wake"]
emit = ["my-plugin:something-happened"]
```

### Config fields

```toml
[[config.fields]]
key = "interval_min"
label = "Interval"
description = "Minutes between checks"
type = "integer"
default = 30
min = 5
max = 120
```

### Data providers

```toml
[[data.providers]]
name = "my_status"
description = "Current plugin status"
schema = '{"type":"object"}'
```

### UI panels

```toml
[[ui.panels]]
label = "panel-my-plugin"
title = "My Plugin"
width = 360
height = 460
entry = "ui/panel.html"
```

## Export Conventions

| Export name | Purpose | Required? |
|---|---|---|
| `plugin_init` | Called when the plugin is loaded. Return `{"status":"ok"}`. | Yes |
| `plugin_shutdown` | Called when the plugin is unloaded. | No |
| `on_event` | Receives subscribed events. Input: `{"event":"...","payload":{...}}` | No |
| `tool_{name}` | Agent-callable tool. Name must match `[[tools.definitions]]`. | No |
| `data_{name}` | Queryable data provider. Name must match `[[data.providers]]`. | No |

## Plugin SDK (Rust)

The `peekoo-plugin-sdk` crate provides safe, typed wrappers for all host functions. Import with:

```rust
use peekoo_plugin_sdk::prelude::*;
```

The prelude re-exports `plugin_fn`, `FnResult`, `Json`, `Error`, `Serialize`, `Deserialize`, `Value`, and the `peekoo` namespace.

### State — `peekoo::state`

Key-value store scoped to the current plugin. Requires `state:read` / `state:write` permissions.

```rust
let count: u64 = peekoo::state::get("call_count")?.unwrap_or(0);
peekoo::state::set("call_count", &(count + 1))?;
peekoo::state::delete("old_key")?;
```

### Logging — `peekoo::log`

Messages are tagged with the plugin key and routed to the app's tracing subscriber.

```rust
peekoo::log::info("something happened");
peekoo::log::warn("something looks off");
peekoo::log::error("something broke");
peekoo::log::debug("detailed info");
```

### Notifications — `peekoo::notify`

Requires the `notifications` permission.

```rust
let delivered = peekoo::notify::send("Reminder", "Time to stretch")?;
```

Returns `true` if delivered, `false` if suppressed (e.g. by do-not-disturb).

### Scheduling — `peekoo::schedule`

Requires the `scheduler` permission. Fires `schedule:fired` events.

```rust
// Repeating timer, 5 minutes
peekoo::schedule::set("my_timer", 300, true, None)?;

// Check remaining time
if let Some(info) = peekoo::schedule::get("my_timer")? {
    peekoo::log::info(&format!("{}s left", info.time_remaining_secs));
}

// Cancel
peekoo::schedule::cancel("my_timer")?;
```

### Config — `peekoo::config`

Reads resolved config values (user overrides merged with defaults from the manifest).

```rust
let interval: u64 = peekoo::config::get("interval_min")?.unwrap_or(30);
let all_config = peekoo::config::get_all()?;
```

### Badge — `peekoo::badge`

Set items on the Peek overlay. Requires the `notifications` permission.

```rust
peekoo::badge::set(&[
    BadgeItem {
        label: "Water".into(),
        value: "~5 min".into(),
        icon: Some("droplet".into()),
        countdown_secs: Some(300),
    },
])?;
```

### Events — `peekoo::events`

Emit custom events that other plugins can subscribe to.

```rust
peekoo::events::emit("my-plugin:task-done", serde_json::json!({ "task": "cleanup" }))?;
```

### Types

```rust
pub struct ScheduleInfo {
    pub owner: String,
    pub key: String,
    pub interval_secs: u64,
    pub repeat: bool,
    pub time_remaining_secs: u64,
}

pub struct BadgeItem {
    pub label: String,
    pub value: String,
    pub icon: Option<String>,
    pub countdown_secs: Option<u64>,
}
```

## Plugin SDK (AssemblyScript)

The `@peekoo/plugin-sdk` package provides typed wrappers for all host functions. Import individual modules:

```typescript
import * as state from "@peekoo/plugin-sdk/assembly/state";
import * as log from "@peekoo/plugin-sdk/assembly/log";
import * as notify from "@peekoo/plugin-sdk/assembly/notify";
import * as schedule from "@peekoo/plugin-sdk/assembly/schedule";
import * as config from "@peekoo/plugin-sdk/assembly/config";
import * as badge from "@peekoo/plugin-sdk/assembly/badge";
import * as events from "@peekoo/plugin-sdk/assembly/events";
```

### State

```typescript
const count = state.get("call_count");       // returns string
state.set("call_count", "42");
state.del("old_key");
```

### Logging

```typescript
log.info("hello");
log.warn("careful");
log.error("broken");
log.debug("details");
```

### Notifications

```typescript
const delivered = notify.send("Title", "Body");  // returns bool
```

### Scheduling

```typescript
schedule.set("timer", 300, true);             // key, interval_secs, repeat
schedule.cancel("timer");
const info = schedule.get("timer");           // ScheduleInfo | null
```

### Config

```typescript
const value = config.get("interval_min");     // returns string
const all = config.getAll();                  // returns the raw config object JSON string
```

### Badge

```typescript
import { BadgeItem } from "@peekoo/plugin-sdk/assembly/types";
const item = new BadgeItem();
item.label = "Water";
item.value = "~5 min";
item.icon = "droplet";
item.countdown_secs = 300;
badge.set([item]);
```

### Events

```typescript
events.emit("my-plugin:done", '{"task":"cleanup"}');
```

## Host Functions Reference

All 10 host functions available to plugins:

| Host function | JSON input | JSON output | SDK module |
|---|---|---|---|
| `peekoo_state_get` | `{"key":"..."}` | `{"value": any}` | `state` |
| `peekoo_state_set` | `{"key":"...","value": any}` | `{"ok": bool}` | `state` |
| `peekoo_log` | `{"level":"info\|warn\|error\|debug","message":"..."}` | `{"ok": true}` | `log` |
| `peekoo_emit_event` | `{"event":"...","payload": any}` | `{"ok": true}` | `events` |
| `peekoo_notify` | `{"title":"...","body":"..."}` | `{"ok": true, "suppressed": bool}` | `notify` |
| `peekoo_schedule_set` | `{"key":"...","interval_secs": u64,"repeat": bool,"delay_secs"?: u64}` | `{"ok": bool}` | `schedule` |
| `peekoo_schedule_cancel` | `{"key":"..."}` | `{"ok": true}` | `schedule` |
| `peekoo_schedule_get` | `{"key":"..."}` | `{"schedule": null \| ScheduleInfo}` | `schedule` |
| `peekoo_config_get` | `{"key"?: "..."}` | `{"value": any}` | `config` |
| `peekoo_set_peek_badge` | `[BadgeItem, ...]` (raw JSON array) | `{"ok": true}` | `badge` |

## Permissions Reference

| Permission | Required for |
|---|---|
| `state:read` | `peekoo_state_get` |
| `state:write` | `peekoo_state_set` |
| `scheduler` | `peekoo_schedule_set`, `peekoo_schedule_cancel`, `peekoo_schedule_get` |
| `notifications` | `peekoo_notify`, `peekoo_set_peek_badge` |
| `bridge:fs_read` | `peekoo_bridge_fs_read` |
| `pet:mood` | `peekoo_set_mood` |
| `agent:register-tool` | Registering tools with the LLM agent |

Permissions are enforced twice: the plugin must declare the capability in
`peekoo-plugin.toml`, and the capability must also be granted by the host.
Calling a gated host function without both declaration and grant fails at
runtime.

## System Events

| Event name | Payload | Fired when |
|---|---|---|
| `schedule:fired` | `{"key": "<schedule_key>"}` | A scheduled timer fires |
| `system:wake` | `{}` | System resumes from sleep/suspend |

## Example Plugins

- `plugins/example-minimal/` — smallest Rust plugin with one tool (uses SDK)
- `plugins/health-reminders/` — full-featured: state, events, notifications, scheduling, tools, data, UI
- `plugins/as-example-minimal/` — smallest AssemblyScript plugin with one tool (uses SDK)

## Building

```bash
# Check the SDK compiles
just check-sdk

# Build a Rust plugin
just plugin-build example-minimal

# Build an AssemblyScript plugin
just plugin-build-as as-example-minimal

# Build all examples
just plugin-build-all

# Build and install a Rust plugin
just plugin example-minimal
```

## Current Limitations

- Plugin panels are rendered by the frontend from HTML returned by Tauri
- Required permissions are auto-granted during startup install for local development
- `@peekoo/plugin-sdk` (AssemblyScript) is a local path dependency, not published to npm
- No unit testing harness for plugins (must test via the full app runtime)
