# Agent-First Architecture

## Intent

Peekoo treats agent behavior as a first-class product capability. The architecture centers on agent runtime, provider/auth configuration, and predictable command transport.

## Dependency Flow

`apps/desktop-ui` -> `apps/desktop-tauri/src-tauri` -> `crates/peekoo-agent-app` -> (`crates/peekoo-agent`, `crates/peekoo-agent-auth`, `crates/persistence-sqlite`, `crates/security`)

`crates/peekoo-productivity-domain` provides shared task/pomodoro domain models for productivity features.

## Responsibilities

- `peekoo-agent`: prompt/session runtime facade and model operations.
- `peekoo-agent-auth`: OAuth protocol logic and provider auth flow.
- `peekoo-agent-app`: application orchestration for settings, auth, provider config, and runtime lifecycle.
- `peekoo-productivity-domain`: task and pomodoro domain entities/state transitions.
- `desktop-tauri`: command transport, DTO mapping, window event emission.

## Boundaries

- No persistence/OAuth/runtime business logic in Tauri command handlers.
- No UI/Tauri dependencies in `peekoo-agent*` crates.
- No reintroduction of `core-app`; domain-specific app crates own orchestration.
