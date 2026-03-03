# AGENTS.md - peekoo-event-bus

## Overview
Async pub/sub event bus for decoupled communication between components. Uses Tokio broadcast channels.

## Key Types
- `EventEnvelope` - Event wrapper with trace_id, event_type, schema_version, and payload
- `EventBus` - Broadcast channel-based event bus

## Dependencies
- `serde` / `serde_json` - Serialization
- `tokio` (sync feature) - Async runtime

## Usage Example
```rust
let bus = EventBus::new(8);
let mut rx = bus.subscribe();

bus.publish(EventEnvelope {
    trace_id: "trace-1".to_string(),
    event_type: "task.created".to_string(),
    schema_version: "v1".to_string(),
    payload: json!({"id": "task-1"}),
})?;

let event = rx.recv().await?;
```

## Testing
```bash
cargo test -p peekoo-event-bus
```

## Code Style
- Use broadcast channels for 1:N communication
- Include trace_id for request tracking
- Version event schemas
