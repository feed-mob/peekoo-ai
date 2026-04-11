# Logging System Design

**Date**: 2026-03-09
**Status**: Approved

## Problem

The codebase has no persistent logging. Errors are difficult to debug in production because:
- `tracing::*` macros are called but no subscriber is initialized (logs are silently discarded)
- `eprintln!` goes nowhere in release builds (console window hidden on Windows)
- Frontend uses ad-hoc `console.*` calls that aren't persisted

## Solution

Implement unified logging using `tauri-plugin-log` with:
- File persistence with size-based rotation
- Error-level by default in production
- Trace-level by default in development (`just dev`)
- `RUST_LOG` env var for runtime level control
- Frontend-to-backend log bridging

## Architecture

```
Frontend (React)                    Backend (Rust)
 console.error()  ──┐          tracing::error!()  ──┐
 console.warn()   ──┤          tracing::info!()   ──┤
 console.log()    ──┤          tracing::debug!()  ──┤
                    │                                │
         interceptConsole()              tauri-plugin-log subscriber
                    │                                │
                    └──────────┬─────────────────────┘
                               │
                    tauri-plugin-log backend
                         │              │
                    ┌────┘              └────┐
              File target              Stdout
         (./logs/ or LogDir)
```

## Configuration

| Setting | Value |
|---------|-------|
| Default level (production) | Error |
| Default level (`just dev`) | Trace |
| Runtime override | `RUST_LOG` env var |
| Max file size | 5 MB |
| Rotation | Keep last 5 files |
| Log directory (dev) | `<project-root>/logs/` |
| Log directory (production) | Platform `LogDir` (via tauri-plugin-log) |

### Log File Locations

**Development** (`just dev`):
- All platforms: `<project-root>/logs/Peekoo.log`

**Production** (release builds):
- Linux: `~/.local/share/com.peekoo.desktop/logs/`
- macOS: `~/Library/Logs/com.peekoo.desktop/`
- Windows: `%LOCALAPPDATA%/com.peekoo.desktop/logs/`

Dev mode uses `cfg!(debug_assertions)` to detect build profile and `PEEKOO_PROJECT_ROOT` env var (set by justfile) to resolve the project root.

## Implementation

### Files Changed

1. `apps/desktop-tauri/src-tauri/Cargo.toml` - Add dependencies
2. `apps/desktop-tauri/src-tauri/src/lib.rs` - Initialize plugin
3. `apps/desktop-tauri/src-tauri/capabilities/default.json` - Add permission
4. `apps/desktop-ui/package.json` - Add frontend dependency
5. `apps/desktop-ui/src/main.tsx` - Intercept console
6. `crates/peekoo-paths/src/lib.rs` - Add `peekoo_log_dir()` helper
7. `justfile` - Set `RUST_LOG=trace` for dev command

### Dependencies

**Rust** (`desktop-tauri`):
- `tauri-plugin-log = { version = "2", features = ["tracing"] }`
- `log = "0.4"`

**Frontend** (`desktop-ui`):
- `@tauri-apps/plugin-log`

## Usage

```bash
# Development (trace level by default)
just dev

# Production (error level by default)
just build

# Override level at runtime
RUST_LOG=debug just dev
RUST_LOG=peekoo_plugin_host=trace just dev
```

## Future Considerations

- "Show logs" button in settings UI
- Log viewer panel
- Telemetry/error reporting integration
