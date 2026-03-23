## 2026-03-23 23:17: feat: Agent Task Execution via ACP

**What changed:**
- Implemented full task assignment to AI agents using ACP (Agent Client Protocol)
- Created `peekoo-agent-acp` binary crate implementing ACP Agent trait for JSON-RPC communication over stdio
- Added agent registry table (`agent_registry`) with default "peekoo-agent" entry
- Added task work tracking columns to `tasks` table: `agent_work_status`, `agent_work_session_id`, `agent_work_attempt_count`, `agent_work_started_at`, `agent_work_completed_at`
- Implemented `AgentScheduler` service with 30-second polling interval for task execution
- Integrated AgentScheduler into `AgentApplication` lifecycle (instantiated in `new()`, started in `start_plugin_runtime()`)
- Full ACP subprocess communication: initialize → new_session → prompt flow
- Added comprehensive logging throughout the agent execution pipeline for debugging
- Updated frontend Task type to support string-based assignee IDs ("user", "peekoo-agent")
- Added agent selector dropdown in task detail view with "Me" and "Peekoo Agent" options

**Why:**
- Enable AI agents to automatically work on assigned tasks based on scheduled time
- Provide foundation for future multi-agent system with agent registry and capabilities
- Give users visibility into agent task execution via comprehensive logging
- Allow agents to decide how to handle tasks (auto-complete, ask questions, or create plans)

**Files affected:**
- `crates/peekoo-agent-acp/` - New ACP server binary crate
- `crates/peekoo-agent-app/src/agent_scheduler.rs` - Agent task scheduler implementation
- `crates/peekoo-agent-app/src/application.rs` - AgentScheduler integration
- `crates/peekoo-agent-app/src/productivity.rs` - Task claiming and work status methods
- `crates/peekoo-productivity-domain/src/task.rs` - AgentWorkStatus enum and TaskDto fields
- `crates/persistence-sqlite/migrations/0009_agent_task_assignment.sql` - Database migration
- `crates/persistence-sqlite/src/lib.rs` - Migration export
- `crates/peekoo-agent-app/src/settings/store.rs` - Migration execution
- `apps/desktop-ui/src/types/task.ts` - Frontend Task type updates
- `apps/desktop-ui/src/features/tasks/components/TaskDetailView.tsx` - Agent selector UI
- `apps/desktop-ui/src/features/tasks/components/TaskListItem.tsx` - Assignee icon update
- `docs/plans/2026-03-23-task-agent-assignment.md` - Design document

**Database Migration:**
- Migration 0009: Updates assignee from "agent" to "peekoo-agent", adds agent work tracking columns, creates agent_registry table with index

**Testing:**
- All 170 tests passing
- Clippy clean with no warnings
- PR #125 created: https://github.com/feed-mob/peekoo-ai/pull/125
