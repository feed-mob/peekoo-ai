# AGENTS.md - peekoo-security

## Overview
Security utilities for secret storage and redaction.

## Key Types
- `SecretStore` trait - Interface for secure secret storage
- `InMemorySecretStore` - In-memory implementation for testing
- `KeyringSecretStore` - OS keychain-backed implementation
- `FileSecretStore` - Local filesystem fallback with restrictive permissions
- `FallbackSecretStore` - Composite store that falls back when the primary is unavailable
- `SecretStoreError` - Error types (NotFound, Unavailable)

## Key Functions
- `redact_secret(input: &str) -> String` - Redact secrets, showing only last 4 chars

## Dependencies
- `thiserror` - Error handling
- `keyring` - Native credential storage integration

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
- Prefer fallback-based composition when platform keychains may be unavailable
- Provide in-memory implementation for tests
