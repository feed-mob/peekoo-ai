## 2026-03-24 09:30: fix: Trigger agent follow-ups immediately from comments

**What changed:**
- Added immediate follow-up triggering from task comments that mention `@peekoo-agent`
- Reset stuck agent tasks from `executing` back to `pending` when a new explicit follow-up comment arrives
- Added `AgentScheduler::trigger_now()` so follow-up comments can launch execution immediately instead of waiting for the polling interval
- Tightened task execution context so only real comment events are included, in chronological order, with the latest follow-up highlighted
- Enabled per-task persistent ACP/agent session reuse to preserve follow-up context across runs

**Why:**
- Follow-up comments were not waking the scheduler promptly and could be blocked forever by stale `agent_work_status = executing`
- The agent needed cleaner comment-only context and latest-comment prioritization to answer follow-up requests instead of repeating the original task

**Files affected:**
- `crates/peekoo-agent-app/src/productivity.rs`
- `crates/peekoo-agent-app/src/task_runtime_service.rs`
- `crates/peekoo-agent-app/src/agent_scheduler.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-acp/src/context.rs`
- `crates/peekoo-agent-acp/src/agent.rs`
