# 2026-03-31 Session Resumption and Session Type Filtering

## Problem
1. **Session resumption broken**: When reopening the app, follow-up questions created a new session instead of continuing the conversation. The session ID was stored in the wrong config field (`session_path` instead of `resume_session_id`).
2. **ACP task sessions mixed with chat**: Task scheduler sessions appeared in the chat panel's "last session" loading, cluttering the conversation history.

## Solution

### Database
- Added `session_type` column (`chat` / `acp_task`) to `agent_sessions` with index
- Added `acp_session_id` column to persist the ACP agent's internal session ID

### Session Resumption (Hybrid Strategy)
- **Same provider**: Uses ACP's native `session/resume` (preferred) or `session/load` to restore context
- **Provider switched**: Replays conversation history into the new ACP agent via `prompt_with_history`
- ACP session ID persisted after creation/load/resume for future resumption
- Event callback suppressed during history replay to avoid UI flooding

### Code Changes
- `SessionType` enum (`Chat` / `AcpTask`) for type-safe session classification
- `AcpBackend` captures `load_session`/`resume_session` capabilities from `InitializeResponse`
- New `LoadSession` and `ResumeSession` ACP commands with worker thread handlers
- `AgentBackend` trait extended with `prompt_with_history`, `get_acp_session_id`, and session resumption methods
- `AgentService::resume_session()` rewritten with hybrid logic and provider change detection
- `peekoo-agent-acp` marks its sessions as `acp_task`
- `conversation.rs` filters last session to `SessionType::Chat` only

### Files Changed
- `crates/persistence-sqlite/migrations/202603310002_add_session_type.sql` (new)
- `crates/persistence-sqlite/migrations/202603310003_add_acp_session_id.sql` (new)
- `crates/peekoo-agent/src/session_store.rs`
- `crates/peekoo-agent/src/config.rs`
- `crates/peekoo-agent/src/service.rs`
- `crates/peekoo-agent/src/backend/mod.rs`
- `crates/peekoo-agent/src/backend/acp.rs`
- `crates/peekoo-agent/src/lib.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/conversation.rs`
- `crates/peekoo-agent-acp/src/agent.rs`
