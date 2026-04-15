# Plugin SDK

**Date**: 2026-03-12
**Status**: Implemented

## Problem

Every Peekoo plugin duplicates ~60-100 lines of boilerplate:
- All 10 `extern "ExtismHost"` function declarations
- Matching request/response structs (`StateGetRequest`, `LogRequest`, `ScheduleSetRequest`, etc.)
- Safe wrapper functions around `unsafe` host calls

There is no shared crate for plugin authors. Plugins copy structs from `health-reminders` and write `unsafe` blocks at every call site. Additional issues:

1. `peekoo_set_peek_badge` has an inconsistent signature (raw `String` instead of `Json<T>`)
2. `docs/plugin-authoring.md` says `wasm32-unknown-unknown` but plugins use `wasm32-wasip1`
3. Only 5 of 10 host functions are documented
4. `plugin_shutdown` lifecycle hook exists in runtime but is undocumented
5. No AssemblyScript path exists for TypeScript-preferring plugin authors
6. No `cargo-generate` template for scaffolding new plugins

## Solution

Build a Plugin SDK spanning Rust and AssemblyScript:
- **Rust SDK crate** (`peekoo-plugin-sdk`) — typed, safe wrappers for all 10 host functions
- **AssemblyScript SDK package** (`@peekoo/plugin-sdk`) — wrapper around `@extism/as-pdk`
- **Rewrite `example-minimal`** using the SDK to prove the API
- **`cargo-generate` template** for new Rust plugins
- **Full docs rewrite** fixing all known inaccuracies

## Architecture

### Rust SDK — `crates/peekoo-plugin-sdk/`

Isolated workspace (targets `wasm32-wasip1`, not in main workspace members).

```
crates/peekoo-plugin-sdk/
  Cargo.toml
  .cargo/config.toml        # target = "wasm32-wasip1"
  src/
    lib.rs                   # module declarations, prelude, re-exports
    types.rs                 # ScheduleInfo, BadgeItem, LogLevel
    host_fns.rs              # private: all 10 extern "ExtismHost" + request/response structs
    state.rs                 # state::get<T>(), set<T>(), delete()
    log.rs                   # log::info/warn/error/debug()
    notify.rs                # notify::send(title, body)
    schedule.rs              # schedule::set/cancel/get()
    config.rs                # config::get<T>(key), get_all()
    badge.rs                 # badge::set(items) — papers over raw-string host ABI
    events.rs                # events::emit(name, payload), SystemEvent enum
```

**Dependencies:**
- `extism-pdk = "1.4"`
- `serde = { version = "1", features = ["derive"] }`
- `serde_json = "1"`

### Public API

```rust
use peekoo_plugin_sdk::prelude::*;

// State
peekoo::state::get::<T>("key")          -> Result<Option<T>, Error>
peekoo::state::set("key", &value)       -> Result<(), Error>
peekoo::state::delete("key")            -> Result<(), Error>

// Logging
peekoo::log::info("msg")
peekoo::log::warn("msg")
peekoo::log::error("msg")
peekoo::log::debug("msg")

// Notifications
peekoo::notify::send("title", "body")   -> Result<bool, Error>

// Scheduling
peekoo::schedule::set(key, interval_secs, repeat, delay_secs) -> Result<(), Error>
peekoo::schedule::cancel(key)           -> Result<(), Error>
peekoo::schedule::get(key)              -> Result<Option<ScheduleInfo>, Error>

// Config
peekoo::config::get::<T>("key")         -> Result<Option<T>, Error>
peekoo::config::get_all()               -> Result<Value, Error>

// Badge
peekoo::badge::set(items: &[BadgeItem]) -> Result<(), Error>

// Events
peekoo::events::emit(name, payload)     -> Result<(), Error>
```

**Prelude re-exports:** `extism_pdk::{plugin_fn, FnResult, Json, Error}` + all `peekoo::*` modules.

**Types:**
```rust
pub struct ScheduleInfo {
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

pub enum SystemEvent {
    ScheduleFired,
    SystemWake,
}
```

### Host Function Mapping

All 10 host functions are wrapped:

| Host function | SDK module | Notes |
|---|---|---|
| `peekoo_state_get` | `state::get` | Deserializes `Value` to `T` |
| `peekoo_state_set` | `state::set` | Serializes `T` to `Value` |
| `peekoo_log` | `log::*` | Level mapped from function name |
| `peekoo_emit_event` | `events::emit` | Accepts any serializable payload |
| `peekoo_notify` | `notify::send` | Returns `bool` (suppressed or not) |
| `peekoo_schedule_set` | `schedule::set` | `delay_secs` defaults to 0 |
| `peekoo_schedule_cancel` | `schedule::cancel` | — |
| `peekoo_schedule_get` | `schedule::get` | Returns `Option<ScheduleInfo>` |
| `peekoo_config_get` | `config::get` / `config::get_all` | Key omitted = all config |
| `peekoo_set_peek_badge` | `badge::set` | Papers over raw-string inconsistency |

### AssemblyScript SDK — `packages/plugin-sdk/`

npm package: `@peekoo/plugin-sdk`

```
packages/plugin-sdk/
  package.json               # name: @peekoo/plugin-sdk, dep: @extism/as-pdk
  assembly/
    index.ts                 # re-exports all peekoo wrappers
    state.ts                 # state.get<T>(), set<T>(), delete()
    log.ts                   # log.info/warn/error/debug()
    notify.ts                # notify.send(title, body)
    schedule.ts              # schedule.set/cancel/get()
    config.ts                # config.get<T>(key), getAll()
    badge.ts                 # badge.set(items)
    events.ts                # events.emit(name, payload)
    types.ts                 # ScheduleInfo, BadgeItem
  tsconfig.json
  README.md
```

Local path dependency for v1 (not published to npm). Plugins reference it as:
```json
{ "@peekoo/plugin-sdk": "file:../../packages/plugin-sdk" }
```

### AssemblyScript Example — `plugins/as-example-minimal/`

```
plugins/as-example-minimal/
  package.json               # deps: @extism/as-pdk, @peekoo/plugin-sdk (path)
  asconfig.json              # asc compiler config
  assembly/
    index.ts                 # echo tool using @peekoo/plugin-sdk
  peekoo-plugin.toml         # plugin manifest (same schema as Rust plugins)
  README.md
```

### Rust Template — `plugins/template-rust/`

`cargo-generate` template for scaffolding new Rust plugins.

```
plugins/template-rust/
  cargo-generate.toml        # [template] substitution vars: plugin_name, plugin_key
  Cargo.toml                 # {{crate_name}}, cdylib, peekoo-plugin-sdk path dep
  src/lib.rs                 # minimal plugin_init + one tool with {{plugin_name}} placeholders
  peekoo-plugin.toml         # manifest with {{plugin_key}} placeholders
  .cargo/config.toml         # target = "wasm32-wasip1"
  README.md
```

Usage: `cargo generate --path plugins/template-rust --name my-plugin`

## Before / After

### Before (example-minimal — 73 lines)

```rust
use extism_pdk::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ~40 lines of host_fn declarations, structs, unsafe wrappers
#[host_fn] extern "ExtismHost" { fn peekoo_state_get(...); fn peekoo_state_set(...); }
struct StateGetRequest { key: String }
struct StateGetResponse { value: Value }
struct StateSetRequest { key: String, value: Value }
fn state_get_u64(key: &str) -> u64 { unsafe { ... } }
fn state_set(key: &str, value: &Value) { unsafe { ... } }
// ... etc

#[plugin_fn]
pub fn tool_example_echo(Json(input): Json<EchoInput>) -> FnResult<Json<EchoOutput>> { ... }
```

### After (example-minimal — ~25 lines)

```rust
use peekoo_plugin_sdk::prelude::*;

#[derive(Deserialize)]
struct EchoInput { input: String }

#[derive(Serialize)]
struct EchoOutput { echo: String, call_count: u64 }

#[plugin_fn]
pub fn plugin_init(_: String) -> FnResult<String> {
    Ok(r#"{"status":"ok"}"#.into())
}

#[plugin_fn]
pub fn tool_example_echo(Json(req): Json<EchoInput>) -> FnResult<Json<EchoOutput>> {
    let count: u64 = peekoo::state::get("call_count")?.unwrap_or(0);
    peekoo::state::set("call_count", &(count + 1))?;
    peekoo::log::info("tool_example_echo called");
    Ok(Json(EchoOutput { echo: req.input, call_count: count + 1 }))
}
```

## Files Changed

### New files

| Path | Purpose |
|---|---|
| `crates/peekoo-plugin-sdk/Cargo.toml` | Isolated workspace crate |
| `crates/peekoo-plugin-sdk/.cargo/config.toml` | Default WASM target |
| `crates/peekoo-plugin-sdk/src/lib.rs` | Module declarations, prelude |
| `crates/peekoo-plugin-sdk/src/types.rs` | Shared types |
| `crates/peekoo-plugin-sdk/src/host_fns.rs` | Private host function declarations |
| `crates/peekoo-plugin-sdk/src/state.rs` | State API |
| `crates/peekoo-plugin-sdk/src/log.rs` | Log API |
| `crates/peekoo-plugin-sdk/src/notify.rs` | Notify API |
| `crates/peekoo-plugin-sdk/src/schedule.rs` | Schedule API |
| `crates/peekoo-plugin-sdk/src/config.rs` | Config API |
| `crates/peekoo-plugin-sdk/src/badge.rs` | Badge API |
| `crates/peekoo-plugin-sdk/src/events.rs` | Events API |
| `plugins/template-rust/*` | `cargo-generate` template |
| `plugins/as-example-minimal/*` | AssemblyScript example plugin |
| `packages/plugin-sdk/*` | `@peekoo/plugin-sdk` npm package |

### Edited files

| Path | Change |
|---|---|
| `plugins/example-minimal/src/lib.rs` | Rewrite using SDK (~25 lines) |
| `plugins/example-minimal/Cargo.toml` | Add `peekoo-plugin-sdk` path dep |
| `docs/plugin-authoring.md` | Full rewrite (fix target, document all 10 host fns, both paths) |
| `justfile` | Add `check-sdk`, `plugin-build-as`, update `plugin-build-all` |

### Intentionally unchanged

- `Cargo.toml` (workspace root) — SDK is its own isolated workspace
- `plugins/health-reminders/` — migration deferred to follow-up PR
- Main workspace crates — no changes needed

## Justfile Additions

```just
# Check the plugin SDK (wasm32-wasip1 target)
check-sdk:
    cargo check --manifest-path crates/peekoo-plugin-sdk/Cargo.toml

# Build AssemblyScript plugin example
plugin-build-as name:
    cd plugins/{{name}} && bun install && bun run build

# Build all plugin examples including AS
plugin-build-all:
    just plugin-build example-minimal
    just plugin-build health-reminders
    just plugin-build-as as-example-minimal
```

## Permissions Reference

| Permission | Used by |
|---|---|
| `state:read` | `state::get` |
| `state:write` | `state::set`, `state::delete` |
| `scheduler` | `schedule::set`, `schedule::cancel`, `schedule::get` |
| `notifications` | `notify::send`, `badge::set` |
| `agent:register-tool` | Registering tools with the LLM agent |

## System Events Reference

| Event | Fired when |
|---|---|
| `schedule:fired` | A scheduled timer fires |
| `system:wake` | System resumes from sleep/suspend |

## Future Considerations

- Migrate `health-reminders` plugin to use the SDK
- Publish `@peekoo/plugin-sdk` to npm when there are multiple AS plugins
- Add `just plugin-new` CLI scaffolding command
- Plugin testing utilities (mock host functions for unit testing without runtime)
