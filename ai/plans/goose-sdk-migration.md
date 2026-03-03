# Plan: Goose SDK vs Pi Agent SDK — Comparison & Migration

## Overview

Assessment of whether **Block’s Goose** SDK can replace **Pi Agent Rust** (`pi_agent_rust`) in `peekoo-agent`, plus a direct comparison and migration options.

---

## Can You Replace Pi with Goose?

**Yes, with caveats.**

- Goose is **embeddable**: it exposes `goose::agents::Agent`, session management, and `agent.reply(user_message, session_config, None)` returning a stream of `AgentEvent`. The repo includes an [agent example](https://github.com/block/goose/blob/main/crates/goose/examples/agent.rs) showing library-style usage.
- **But** Block’s Goose is **not on crates.io**. The crate named [goose on crates.io](https://crates.io/crates/goose) is a different project (load testing). You must depend on Goose via git, e.g. `goose = { git = "https://github.com/block/goose" }`, and accept that the public API may change with the main app.
- Migration is **non-trivial**: Pi’s API is a thin session handle + prompt callback; Goose’s is session manager + provider + extensions + streaming. You’d implement an adapter in `peekoo-agent` that keeps your current `AgentService` / `AgentServiceConfig` surface and backs it with Goose instead of Pi.

---

## Side-by-Side Comparison

| Aspect | Pi Agent Rust (`pi_agent_rust`) | Block Goose |
|--------|----------------------------------|-------------|
| **Distribution** | Published on crates.io (`pi_agent_rust` 0.1.7) | Not on crates.io; use `git = "https://github.com/block/goose"` |
| **Runtime** | **asupersync** (custom async runtime) | **tokio** |
| **Session API** | `create_agent_session(SessionOptions)` → `AgentSessionHandle` | `Agent::new()` (or `with_config`), then `session_manager.create_session(...)` |
| **Sending a prompt** | `handle.prompt(input, on_event)` → returns `AssistantMessage`; events via callback | `agent.reply(user_message, session_config, None)` → returns `BoxStream` of `AgentEvent` |
| **Provider/model** | `SessionOptions { provider, model, ... }`; `handle.set_model(provider, model)` | `create_with_named_model(...)` → provider; `agent.update_provider(provider, &session.id)` |
| **Config / auth** | `~/.pi/agent/auth.json`, `settings.json`; env vars | Goose `Config::global()`, keyring, env; more providers (Bedrock, Vertex, Databricks, etc.) |
| **Tools** | Built-in: read, write, edit, bash, grep, find, ls; AgentSkills (markdown → system prompt) | MCP-based extensions, platform tools, “developer” extension (CLI subprocess); richer tool model |
| **Streaming** | Callback `on_event(AgentEvent)` during `prompt()` | Stream of `AgentEvent` (e.g. `AgentEvent::Message(message)`) |
| **Tauri integration** | Requires asupersync: create reactor + runtime, `block_on(agent.prompt(...))` inside Tauri commands | Native tokio; no asupersync bridge; can use normal async in Tauri |
| **Dependencies** | Lighter (asupersync, pi-specific) | Heavy (AWS SDKs, tree-sitter, Whisper, SQLite, many providers) |
| **Backing** | Community (Dicklesworthstone/pi_agent_rust) | Block (Square, Cash App, Tidal); large OSS project |

---

## Pros and Cons for Peekoo

### Pi Agent Rust (current)

- **Pros:** Already integrated; simple API; on crates.io; small dependency footprint; AgentSkills (markdown) match your “skills as instructions” model; 7 built-in tools.
- **Cons:** asupersync forces a runtime bridge in Tauri; less provider variety; smaller ecosystem than Goose.

### Goose

- **Pros:** Tokio-native (cleaner Tauri integration); many providers and MCP; strong vendor backing; recipe system; extensions; stream-based API fits streaming UIs.
- **Cons:** Git-only dependency; API may evolve; heavier build and dependency set; embedding requires wiring SessionManager/PermissionManager/Config (or their defaults); no direct “AgentSkills” — you’d map skills to system prompt or tools yourself.

---

## How to Replace Pi with Goose (high level)

1. **Add Goose as dependency**  
   In `crates/peekoo-agent/Cargo.toml` (and optionally workspace), add:
   ```toml
   goose = { git = "https://github.com/block/goose" }
   ```
   Resolve any name/version conflicts (e.g. if the workspace uses a different `goose`).

2. **Implement a Goose-backed backend in peekoo-agent**  
   - Keep `AgentServiceConfig` and `AgentService` as the public API.
   - Add a second implementation (e.g. `GooseBackend` or feature-gated “goose” impl) that:
     - Builds `goose::agents::Agent` (e.g. `Agent::new()` or `Agent::with_config(...)` with a minimal `AgentConfig`).
     - Creates a session via `agent.config.session_manager.create_session(...)` and sets provider with `create_with_named_model` + `agent.update_provider(provider, &session.id)`.
     - Maps `AgentServiceConfig` (provider, model, system_prompt, working_directory, etc.) onto Goose’s session/config and provider.
     - Implements `prompt` by building a `Message::user().with_text(input)`, calling `agent.reply(message, session_config, None)`, then consuming the stream and collecting text (and optionally forwarding events to the current callback).
     - Implements `set_model` by creating a new provider and calling `agent.update_provider(..., &session.id)`.
   - Optionally: map AgentSkills (markdown) into Goose’s system prompt or instructions so behavior stays similar.

3. **Tauri / desktop-tauri**  
   - Remove asupersync usage for the agent: no reactor, no `block_on(AgentService::new(...))` / `block_on(agent.prompt(...))`.
   - Use tokio async throughout: e.g. `AgentService::new(config).await` and `agent.prompt(...).await` (or equivalent) on the Goose-backed service.
   - Keep the same Tauri commands (`agent_prompt`, `agent_set_model`, `agent_get_model`) with the same signatures; only the implementation inside `peekoo-agent` changes.

4. **core-app**  
   - `AgentUseCases` continues to depend on `AgentService` and `AgentServiceConfig`; no change if the public API of `peekoo-agent` is unchanged.
   - If you add a “backend” enum (Pi vs Goose), core-app might select it via config or feature; otherwise it stays backend-agnostic.

5. **Testing**  
   - Reuse or adapt existing `peekoo-agent` tests for the Goose backend (e.g. `extract_text`-style helpers and any session/prompt tests).
   - Consider a feature flag or env switch to run integration tests with Pi vs Goose.

6. **Optional: keep Pi as fallback**  
   - Use a feature or config to choose backend (e.g. `backend = "pi" | "goose"`). That way you can switch to Goose without dropping Pi until you’re confident.

---

## Recommendation

- **Short term:** Stay on Pi if you value stability and minimal dependency surface; the main pain is the asupersync bridge in Tauri, which is already working.
- **If you want better Tauri fit and more providers:** Implement a Goose-backed path in `peekoo-agent` behind a feature or config, keep the same `AgentService`/`AgentServiceConfig` API, and migrate Tauri to tokio-only agent calls. Prefer a git dependency with a pinned rev/tag once you pick a stable point in Block’s repo.
- **Before committing:** Clone Block’s goose repo, run its agent example, and confirm you can construct `Agent` + session + provider and call `reply` in a minimal binary; then mirror that in `peekoo-agent` behind your existing API.

---

## References

- [Block Goose](https://github.com/block/goose) — main repo  
- [Goose agent example](https://github.com/block/goose/blob/main/crates/goose/examples/agent.rs)  
- [Pi Agent Rust](https://github.com/Dicklesworthstone/pi_agent_rust)  
- Peekoo usage: `crates/peekoo-agent`, `apps/desktop-tauri/src-tauri/src/lib.rs`, `crates/core-app/src/agent_use_cases.rs`
