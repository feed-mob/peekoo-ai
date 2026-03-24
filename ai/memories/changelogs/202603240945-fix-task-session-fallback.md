## 2026-03-24 09:45: fix: Fall back to new task session when no prior session exists

**What changed:**
- Changed ACP task session reuse to resume a legacy per-task `session_path` only when the file actually exists
- Added fallback to create a fresh persistent task-scoped session directory when no prior session file exists
- Prevented `peekoo-agent-acp` from failing with `Session not found` on the first follow-up run for a task

**Why:**
- The previous task session reuse logic always passed an explicit `session_path`, which told `peekoo-agent` to resume that exact file
- When the file did not exist yet, task execution failed before the agent could run
- Task follow-ups should reuse prior context when available, but must still start cleanly when no saved session exists yet

**Files affected:**
- `crates/peekoo-agent-acp/src/agent.rs`
- `crates/peekoo-agent-acp/Cargo.toml`
