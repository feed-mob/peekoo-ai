# AGENTS.md - peekoo-agent

## Overview
AI agent service - wraps the `pi_agent_rust` library (v0.1.7) and provides a simplified, peekoo-specific API for AI interactions.

## Key Features
- Create agent sessions with chosen LLM providers
- Send prompts and stream responses
- Register custom domain-specific tools ("skills")
- Switch providers/models at runtime
- Load startup instruction files (IDENTITY.md, SOUL.md, memory.md/MEMORY.md, memories/*.md, AGENTS.md, USER.md)
- Auto-discover configuration from ~/.peekoo or local .peekoo directories

## Key Types
- `AgentService` - High-level agent service wrapping pi's session handle
- `AgentServiceConfig` - Configuration with provider, model, skills, and persona loading

## Dependencies
- `peekoo-paths` - Shared path discovery for agent config and model locations
- `pi = { package = "pi_agent_rust", version = "0.1.7" }` - Core agent library

## Testing
```bash
cargo test -p peekoo-agent
```

## Code Style
- Re-export key types from `pi::sdk` for convenience
- Document all public APIs with examples
- Use builder-style patterns for configuration
