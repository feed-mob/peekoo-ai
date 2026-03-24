## 2026-03-24 08:45: feat: Agent follow-up mentions and task notifications

**What changed:**
- Routed scheduled task execution through ACP session `mcpServers` instead of relying on scheduler-authored placeholder comments
- Added an ACP-side MCP bridge that connects the shared task MCP server and exposes task-scoped tools to the real `peekoo-agent`
- Removed hardcoded scheduler-visible task comments/labels/status changes; successful task updates now must come from the agent via MCP tools
- Added task-scoped MCP tool injection so the agent no longer has to guess or pass `task_id` for task comments, status updates, or label updates
- Added `@peekoo-agent` mention support for user comments on agent-assigned tasks by re-queueing internal agent work to `pending`
- Added desktop notifications for agent-authored task comments and agent task status changes only

**Why:**
- The scheduler was producing fake task history instead of real agent-authored updates
- ACP needed to pass MCP server configuration in the protocol-native session setup flow
- Agent follow-up comments need a lightweight way to wake the scheduler without creating extra persistence tables
- Notifications should highlight actual agent responses, not user comments or noisy label changes

**Files affected:**
- `crates/peekoo-agent-app/src/agent_scheduler.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/productivity.rs`
- `crates/peekoo-agent-app/src/task_runtime_service.rs`
- `crates/peekoo-agent-acp/src/agent.rs`
- `crates/peekoo-agent-acp/src/context.rs`
- `crates/peekoo-agent-acp/src/mcp_tools.rs`
- `crates/peekoo-agent-acp/src/lib.rs`
- `crates/peekoo-agent-acp/Cargo.toml`
