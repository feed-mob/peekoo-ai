# Example Minimal Plugin

This is the smallest useful Peekoo plugin.

## What it shows

- `peekoo-plugin.toml` manifest structure
- `cdylib` Rust crate compiled to `wasm32-unknown-unknown`
- `plugin_init` lifecycle export
- one tool export: `tool_example_echo`
- host function imports via `#[host_fn] extern "ExtismHost"`
- state persistence with `peekoo_state_get` / `peekoo_state_set`

## Build

```bash
just plugin-build example-minimal
```

## Install

```bash
just plugin example-minimal
```

## Key conventions

- tool exports are named `tool_{name}`
- manifest tool names must match the export suffix exactly
- all host/plugin payloads are JSON
- use `#![no_main]` for WASM plugins
