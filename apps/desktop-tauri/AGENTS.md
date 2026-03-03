# AGENTS.md - desktop-tauri

## Overview
Tauri desktop app shell for Peekoo AI. Provides the native desktop wrapper for the React frontend.

## Tech Stack
- **Framework**: Tauri v2 (Rust)
- **Frontend**: React app (from desktop-ui)
- **Build**: Cargo

## Status
Currently minimal scaffold - placeholder implementation in `src/main.rs`.

## Scripts
```bash
# Development (requires desktop-ui dev server)
tauri dev

# Build production app
tauri build
```

## Project Structure
```
desktop-tauri/
├── Cargo.toml           # Package manifest
├── src/
│   └── main.rs          # Entry point (currently placeholder)
└── src-tauri/           # Tauri configuration (standard location)
```

## Dependencies
See `Cargo.toml` - currently minimal. Will need:
- `tauri` - Core Tauri framework
- `tauri-plugin-shell` - Shell access
- Peekoo workspace crates for business logic

## Integration
This app shell loads the `desktop-ui` frontend. Build process:
1. Build `desktop-ui` to `dist/`
2. Tauri bundles the dist files with the Rust binary

## Future Work
- Implement Tauri commands for backend communication
- Add system tray integration
- Implement global shortcuts
- Window management (minimize to tray, etc.)
