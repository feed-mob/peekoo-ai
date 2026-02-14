# Peekoo AI Tech Spec (MVP Baseline)

## Stack
- Desktop shell: Tauri v2
- UI: React
- Core: Rust crates (domain/app/plugins/calendar/persistence/security/event-bus)
- Plugin compatibility: MCP first + JS bridge whitelist

## Built-in Agent (v1)
- Single-agent runtime
- Event-driven loop: `turn_start -> llm_stream -> tool_calls -> tool_results -> turn_end`
- Hook points: `BeforeLLMCall`, `AfterLLMCall`, `BeforeToolCall`, `AfterToolCall`

## Data Model
- Tasks, pomodoro sessions, conversations/messages
- Calendar accounts/events/sync state
- Plugins, permissions, plugin state
- Event log for replay and diagnostics

## Security
- OAuth token material goes to OS keychain implementation (placeholder trait in MVP baseline)
- DB stores token references only
- Log redaction applied to secrets

## Milestone Status
- Week 1 scaffold completed: workspace layout, core domain tests, plugin timeout/permission tests, schema migration baseline.
