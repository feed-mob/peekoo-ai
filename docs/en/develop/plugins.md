# Plugin Development

## Overview

Peekoo plugins are WASM modules loaded by the plugin host through Extism. Plugins can be written in Rust or AssemblyScript.

## What a Plugin Can Add

- tools callable by the agent runtime
- UI panels rendered in Peekoo
- event subscriptions
- persistent state
- configuration fields

## Quick Start

### Rust

1. Install the target:

```bash
rustup target add wasm32-wasip1
```

2. Create a plugin under `plugins/<name>/`.
3. Add `peekoo-plugin.toml` and your source files.
4. Build and install:

```bash
just plugin-build <name>
just plugin-install <name>
```

### AssemblyScript

1. Create a plugin under `plugins/<name>/`.
2. Install dependencies.
3. Build and install with the `plugin-build-as` and `plugin-install-as` recipes.

## Required Files

Typical plugin files include:

- `peekoo-plugin.toml`
- plugin source code
- built `.wasm`
- optional `ui/` assets

## Manifest Topics

The plugin manifest can define:

- plugin metadata
- permissions
- tool definitions
- event subscriptions
- config fields
- data providers
- UI panels

## Runtime Notes

Plugins are sandboxed. Host functions cover concerns such as logging, state, and HTTP access, subject to permissions.
