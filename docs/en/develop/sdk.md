# SDK

## Scope

Peekoo currently exposes SDK surfaces in both Rust and JavaScript-oriented forms, but the repository's most mature extension story is the plugin SDK.

## Current SDK Surfaces

### Rust plugin SDK

`crates/peekoo-plugin-sdk` provides typed host bindings for plugin authors. It wraps host functions so plugin code does not need to write raw `extern` bindings or unsafe wrappers.

Key areas include:

- state access
- logging
- notifications
- scheduling
- config lookup
- badge updates
- event emission

### AssemblyScript plugin SDK

`packages/plugin-sdk` provides the AssemblyScript side of the plugin SDK and is currently consumed as a local path dependency.

## Relevant Packages

- `crates/peekoo-plugin-sdk`
- `packages/plugin-sdk`

## Typical Use

Use the SDK when you are building a Peekoo plugin under `plugins/<name>/`.

Typical plugin work includes:

1. add the SDK dependency
2. define `peekoo-plugin.toml`
3. implement plugin exports or tool functions
4. build to WASM
5. install and test inside Peekoo

## What the SDK Solves

The SDK removes low-level plugin boilerplate. Instead of wiring host calls manually, plugin authors work with typed helpers and a more direct authoring model.

This matters most when you want to focus on behavior such as:

- reading or storing plugin state
- exposing one or more tools
- emitting events
- rendering a plugin panel
- reacting to schedules or notifications

## Build Notes

- Rust plugins target `wasm32-wasip1`
- AssemblyScript plugins must provide a local `abort(...)` handler to avoid missing `env::abort` at runtime

## Choosing Rust or AssemblyScript

Choose Rust when you want stronger typing, richer crate reuse, or you are already working in the Rust side of the repository. Choose AssemblyScript when you want a TypeScript-like authoring experience and a lighter path for small plugins.

## What to Read Next

For practical extension work, start with [Plugin Development](./plugins.md).
