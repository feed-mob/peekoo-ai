# AGENTS.md - peekoo-plugin-host

## Overview
Plugin system with capability-based security. Provides async plugin execution with timeout handling.

## Key Types
- `Capability` enum - Available capabilities (FileRead, FileWrite, Network, etc.)
- `PluginContext` - Runtime context for plugins
- `Plugin` trait - Interface for implementing plugins
- `PluginError` - Error types with `thiserror`

## Key Functions
- `execute_with_timeout` - Execute async code with configurable timeout

## Dependencies
- `async-trait` - Async trait methods
- `serde` / `serde_json` - Serialization
- `thiserror` - Error handling
- `tokio` (time, macros, rt) - Async runtime

## Testing
```bash
cargo test -p peekoo-plugin-host
```

## Code Style
- Use capability-based security model
- Implement `Plugin` trait for all plugins
- Handle timeouts explicitly
- Serialize plugin data with JSON
