# AGENTS.md - peekoo-security

## Overview
Security utilities for secret storage and redaction.

## Key Types
- `SecretStore` trait - Interface for secure secret storage
- `InMemorySecretStore` - In-memory implementation for testing
- `SecretStoreError` - Error types (NotFound, Unavailable)

## Key Functions
- `redact_secret(input: &str) -> String` - Redact secrets, showing only last 4 chars

## Dependencies
- `thiserror` - Error handling

## Usage Example
```rust
let store = InMemorySecretStore::default();
store.put("api_key", "secret-token")?;
let value = store.get("api_key")?; // "secret-token"

let redacted = redact_secret("super-secret-value");
// "[REDACTED-alue]"
```

## Testing
```bash
cargo test -p peekoo-security
```

## Code Style
- Use trait-based abstraction for secret storage
- Redact secrets in logs and error messages
- Provide in-memory implementation for tests
