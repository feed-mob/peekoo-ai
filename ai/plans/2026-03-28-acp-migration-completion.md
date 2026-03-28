# ACP Migration Completion Plan

**Date:** 2026-03-28  
**Status:** In Progress  
**Goal:** Finish the ACP migration by consolidating onto one SQLite database, wiring real provider management, replacing legacy chat settings, and restoring conversation history from SQLite.

## Scope

- Use `peekoo.sqlite` as the only app database.
- Remove runtime dependence on `agent_sessions.db`.
- Replace mock ACP provider Tauri commands with real app-layer integration.
- Mount the provider UI in Settings.
- Reimplement chat session restore from `agent_sessions` and `session_messages`.

## Implementation Steps

1. **Single DB consolidation**
   - Point `SessionStore` usage at `peekoo.sqlite`.
   - Keep schema creation under `peekoo-persistence-sqlite`.
   - Remove normal chat/session persistence assumptions tied to `agent_sessions.db`.

2. **Provider service refactor**
   - Make `AgentProviderService` safe for Tauri state and async command usage.
   - Prefer shared SQLite locking patterns already used elsewhere in the app.
   - Expose provider operations through the app layer so Tauri remains transport-only.

3. **Real provider transport**
   - Replace all mock provider commands in `apps/desktop-tauri/src-tauri/src/lib.rs`.
   - Return real provider/config/install/test/default state.

4. **UI wiring**
   - Mount `AgentProviderPanel` in Settings.
   - Keep `useAgentProviders` aligned with the real Tauri payloads.
   - Preserve the existing UI structure while replacing placeholder behavior.

5. **Session restore**
   - Reimplement `conversation.rs` against SQLite-backed agent session tables.
   - Return chat-friendly DTOs for the most recent session.

6. **Verification**
   - Add or update tests for provider service, session restore, and Tauri transport paths.
   - Run focused crate tests and then full `cargo test --all`.

## Acceptance Criteria

- The app uses only `peekoo.sqlite` for settings, provider state, and agent session state.
- The Settings view shows the ACP provider panel with real data.
- Provider commands are no longer stubbed or mock-backed.
- Chat restore reads from SQLite-backed sessions.
- Tests pass.
