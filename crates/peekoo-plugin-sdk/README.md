# peekoo-plugin-sdk

Rust SDK for building Peekoo plugins.

It wraps all Peekoo host functions behind safe, typed APIs so plugin authors do not need to hand-write `extern "ExtismHost"` bindings, request/response structs, or `unsafe` wrappers.

## Add it to a plugin

Create your plugin under `plugins/<name>/` and add:

```toml
[dependencies]
peekoo-plugin-sdk = { path = "../../crates/peekoo-plugin-sdk" }
extism-pdk = "1.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Also make sure your plugin has:

```toml
[lib]
crate-type = ["cdylib"]
```

and `.cargo/config.toml`:

```toml
[build]
target = "wasm32-wasip1"
```

## Minimal example

```rust
#![no_main]

use peekoo_plugin_sdk::prelude::*;

#[derive(Deserialize)]
struct EchoInput {
    input: String,
}

#[derive(Serialize)]
struct EchoOutput {
    echoed: String,
    call_count: u64,
}

#[plugin_fn]
pub fn plugin_init(_: String) -> FnResult<String> {
    peekoo::log::info("plugin started");
    Ok(r#"{"status":"ok"}"#.into())
}

#[plugin_fn]
pub fn tool_echo(Json(input): Json<EchoInput>) -> FnResult<Json<EchoOutput>> {
    let call_count: u64 = peekoo::state::get("call_count")?.unwrap_or(0) + 1;
    peekoo::state::set("call_count", &call_count)?;

    Ok(Json(EchoOutput {
        echoed: input.input,
        call_count,
    }))
}
```

## Available modules

- `peekoo::state` - plugin key/value state
- `peekoo::log` - host-routed logging
- `peekoo::notify` - desktop notifications
- `peekoo::schedule` - timers and schedule lookup
- `peekoo::config` - resolved plugin config values
- `peekoo::badge` - Peek overlay badges
- `peekoo::events` - custom event emission

Import everything you need with:

```rust
use peekoo_plugin_sdk::prelude::*;
```

The prelude re-exports:

- `plugin_fn`
- `FnResult`
- `Json`
- `Error`
- `Serialize` / `Deserialize`
- `serde_json`
- `BadgeItem`, `ScheduleInfo`, `SystemEvent`
- `peekoo`

## Build and check

From the repository root:

```bash
just check-sdk
just plugin-build example-minimal
```

See `docs/plugin-authoring.md` and `plugins/example-minimal/` for full usage patterns.
