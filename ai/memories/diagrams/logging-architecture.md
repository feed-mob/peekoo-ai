# Logging Architecture

Unified log pipeline routing both Rust backend and TypeScript frontend messages to a single persistent log file.

```mermaid
flowchart TD
    subgraph Frontend["Frontend (React / TypeScript)"]
        CE["console.error()"]
        CW["console.warn()"]
        CI["console.info()"]
        CD["console.debug()"]
        CL["console.log()"]
    end

    subgraph Backend["Backend (Rust)"]
        TE["tracing::error!()"]
        TW["tracing::warn!()"]
        TI["tracing::info!()"]
        TD["tracing::debug!()"]
        TT["tracing::trace!()"]
    end

    FC["forwardConsole()\nsrc/lib/log.ts"]
    TB["tauri-plugin-log\ntracing bridge"]
    LP["tauri-plugin-log\nsubscriber"]

    CE & CW & CI & CD & CL --> FC
    FC -->|"@tauri-apps/plugin-log\nIPC"| LP
    TE & TW & TI & TD & TT --> TB --> LP

    LP --> FT
    LP --> SO

    subgraph Targets["Log Targets"]
        FT["File target\n(always)"]
        SO["Stdout\n(always)"]
    end

    FT --> DEV["Dev: project-root/logs/Peekoo.log\n(cfg debug_assertions + PEEKOO_PROJECT_ROOT)"]
    FT --> PROD["Production: platform LogDir\n~/.local/share/com.peekoo.desktop/logs/"]
```

## Configuration

| Setting | Value |
|---------|-------|
| Default level — production | `error` |
| Default level — dev (`just dev`) | `trace` |
| Runtime override | `RUST_LOG` env var |
| Max file size | 5 MB |
| Rotation | Keep last 5 files |

## Level Control

```
RUST_LOG=trace just dev          # everything
RUST_LOG=debug just dev          # debug and above
RUST_LOG=peekoo_plugin_host=trace just dev  # per-crate filter
```

## Notes

- `forwardConsole()` preserves original browser console output and sends a copy to the Rust backend
- Format strings (`%s`, `%d`, `%o`, etc.) are expanded before sending so log entries are human-readable
- The `tracing` feature on `tauri-plugin-log` bridges all existing `tracing::*` calls in `peekoo-plugin-host` and `peekoo-agent-app` automatically — no changes needed in those crates
- Dev vs production file path is determined at runtime via `cfg!(debug_assertions)`, not a feature flag
