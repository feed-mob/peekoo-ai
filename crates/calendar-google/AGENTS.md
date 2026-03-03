# AGENTS.md - peekoo-calendar-google

## Overview
Google Calendar OAuth integration with PKCE support. Builds OAuth URLs with proper security parameters.

## Key Functions
- OAuth URL builder with PKCE, state, and code challenge

## Dependencies
- `url = "2"` - URL building

## Testing
```bash
cargo test -p peekoo-calendar-google
```

## Code Style
- Use PKCE for OAuth security
- Include state parameter for CSRF protection
- Build URLs with the `url` crate
