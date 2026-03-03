# AGENTS.md - peekoo-core-domain

## Overview
Core domain models and business logic for Peekoo AI. Contains entities, value objects, and aggregates.

## Key Types
- `Task` - Task entity with `TaskPriority` and `TaskStatus` enums
- `PomodoroSession` - Pomodoro timer with state machine (Idle→Running→Paused→Completed)

## Dependencies
- `thiserror` - Error handling
- `serde` - Serialization

## Testing
```bash
cargo test -p peekoo-core-domain
```

## Code Style
- Use `thiserror` for custom error types
- Derive `serde::Serialize/Deserialize` for domain types
- Document state machines with doc comments
