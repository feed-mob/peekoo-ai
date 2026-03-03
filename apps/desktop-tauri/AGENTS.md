# AGENTS.md - desktop-tauri

## Overview
Tauri transport layer for the Peekoo desktop app.

This package hosts command handlers and window/event wiring only. Business logic belongs in workspace crates, primarily `peekoo-agent-app`.

## Canonical Entry Point
- Rust runtime crate: `apps/desktop-tauri/src-tauri`
- Frontend host: `apps/desktop-ui`

## Responsibilities
- Register Tauri commands and map request/response DTOs.
- Emit UI-facing events (`window.emit`) for streaming updates.
- Delegate orchestration to `peekoo-agent-app`.

## Non-Responsibilities
- Do not implement provider OAuth protocol logic here.
- Do not implement settings persistence logic here.
- Do not implement agent runtime lifecycle policy here.

## Scripts
```bash
# Development (starts desktop-ui via tauri beforeDevCommand)
cd apps/desktop-tauri/src-tauri && cargo tauri dev

# Build production desktop app
cd apps/desktop-tauri/src-tauri && cargo tauri build
```

## Dependency Boundary
- Allowed direct app-layer dependency: `peekoo-agent-app`
- Avoid direct coupling from this layer into lower-level persistence/security/auth crates.
