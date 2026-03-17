# Desktop Tauri Runtime

Tauri runtime for Peekoo desktop.

## Canonical Runtime Crate

Use `apps/desktop-tauri/src-tauri` as the Rust app entrypoint.

## Development

```bash
just dev
```

Or directly:

```bash
cd apps/desktop-tauri/src-tauri
cargo tauri dev --config tauri.macos.conf.json # macOS
# cargo tauri dev                             # non-macOS
```

## Build

```bash
just build
```

Direct build command:

```bash
cd apps/desktop-tauri/src-tauri
cargo tauri build --config tauri.macos.conf.json # macOS
# cargo tauri build                              # non-macOS
```

## Architecture

- `desktop-tauri` is a transport layer.
- Agent and settings orchestration lives in `crates/peekoo-agent-app`.
- OAuth protocol concerns live in `crates/peekoo-agent-auth`.
