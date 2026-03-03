# AGENTS.md - peekoo-productivity-domain

## Overview
Domain models for productivity concerns in Peekoo AI.

## Key Types
- `Task` with `TaskPriority` and `TaskStatus`
- `PomodoroSession` with `PomodoroState` transitions

## Dependencies
- `serde` for serialization
- `thiserror` for domain errors

## Testing
```bash
cargo test -p peekoo-productivity-domain
```

## Boundaries
- Keep this crate pure domain logic.
- No persistence, OAuth, transport, or UI concerns.
