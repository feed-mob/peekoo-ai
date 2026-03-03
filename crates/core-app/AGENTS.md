# AGENTS.md - peekoo-core-app

## Overview
Application use cases layer - orchestrates domain logic with event publishing. Implements the application layer in Clean Architecture.

## Key Components
- `TaskUseCases` - Task management with event bus integration
- `agent_use_cases` - Agent orchestration (chat, streaming, model switching)

## Dependencies
- `peekoo-core-domain` - Domain models
- `peekoo-event-bus` - Pub/sub events
- `peekoo-agent` - AI agent service
- `serde_json` - JSON handling

## Testing
```bash
cargo test -p peekoo-core-app
```

## Code Style
- Use `Result<T, E>` over panics
- Emit domain events after state changes
- Keep use cases focused on single responsibilities
