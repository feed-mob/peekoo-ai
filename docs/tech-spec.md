# Peekoo AI Tech Spec (MVP Baseline)

## Stack
- Desktop shell: Tauri v2
- UI: React
- Core: Rust crates (`peekoo-agent`, `peekoo-agent-app`, `peekoo-agent-auth`, `peekoo-productivity-domain`, `persistence-sqlite`, `security`, `peekoo-paths`)

## Built-in Agent (v1)
- Single-agent runtime wrapping `pi_agent_rust` (v0.1.7)
- Prompt/response via `AgentService::prompt()` with event callback
- Provider/model switching at runtime via `AgentService::set_model()`
- Persona file loading (AGENTS.md, SOUL.md, IDENTITY.md, USER.md, memory files)
- AgentSkills injected as system prompt instructions

## Data Model
- Tasks, pomodoro sessions, conversations/messages (actively used)
- Agent settings, provider auth, provider configs, skills (actively used)
- Calendar accounts/events/sync state, plugins/permissions, event log (legacy schema — tables exist in 0001_init.sql but no Rust code consumes them)

## Security
- OAuth token material goes to OS keychain via `security` crate (`KeyringSecretStore` with `FileSecretStore` fallback)
- DB stores token references only
- Log redaction applied to secrets via `redact_secret()`

## Milestone Status
- MVP scaffold completed: workspace layout, productivity domain with tests, agent service wrapping pi_agent_rust, settings/auth with input validation, 3 schema migrations, security crate with 4 SecretStore implementations, desktop-tauri wired with 19 commands, React UI with chat/tasks/pomodoro views and sprite animations.
