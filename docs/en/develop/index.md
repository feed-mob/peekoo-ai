# Developer Overview

Peekoo is a Tauri desktop application with a React frontend and Rust backend crates.

## Local Development

```bash
just setup
just dev
```

Useful commands:

```bash
just check
just test
just fmt
just lint
just build
```

## Main Areas

- `apps/desktop-ui/`: React + Vite frontend
- `apps/desktop-tauri/`: desktop runtime
- `crates/`: backend crates for agent runtime, auth, productivity, persistence, and security

## Developer Docs

- [SDK](./sdk.md)
- [Plugin Development](./plugins.md)
