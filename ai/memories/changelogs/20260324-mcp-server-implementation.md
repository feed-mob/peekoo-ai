# 2026-03-24 feat: Implement MCP server for agent task execution

## What changed

- Created `peekoo-mcp-server` crate with TCP transport-based MCP server
- Updated `AgentScheduler` to spawn embedded MCP server on dynamic TCP port
- Modified `peekoo-agent-acp` to connect to MCP server and list available tools
- Added TaskService methods: `add_task_label`, `remove_task_label`, `update_task_status`, `load_task`
- Added `Cancelled` variant to `TaskStatus` enum
- Added `NoopTaskService` to `peekoo-productivity-domain` for testing
- Added agent-specific labels to frontend: `agent_working`, `needs_clarification`, `agent_done`, `needs_review`, `agent_failed`

## Why

Enable AI agents to manage tasks through MCP (Model Context Protocol) tools. The agent can now:
- Add comments to tasks
- Update task labels (mark needs_clarification, agent_done, etc.)
- Update task status (in_progress, done, cancelled)

## Architecture

```
AgentScheduler (main app)
  ├─ Starts MCP Server on TCP port (127.0.0.1:49152-65535)
  │   └─ Direct access to ProductivityService/TaskService
  └─ Spawns peekoo-agent-acp subprocess
      └─ ACP protocol over stdio
          └─ Connects to MCP server via PEEKOO_MCP_HOST/PEEKOO_MCP_PORT env vars
```

## Files affected

### New crate
- `crates/peekoo-mcp-server/` - MCP server with task tools (TCP transport)

### Modified
- `crates/peekoo-agent-app/src/agent_scheduler.rs` - MCP server startup, task state management
- `crates/peekoo-agent-app/Cargo.toml` - Added peekoo-mcp-server dependency
- `crates/peekoo-agent-acp/src/agent.rs` - MCP client connection
- `crates/peekoo-agent-acp/src/context.rs` - Added MCP server fields
- `crates/peekoo-agent-acp/Cargo.toml` - Added rmcp dependency
- `crates/peekoo-productivity-domain/src/task.rs` - Added TaskService methods, Cancelled status, NoopTaskService
- `crates/peekoo-agent-app/src/productivity.rs` - Implemented new TaskService methods
- `crates/peekoo-plugin-host/src/host_functions.rs` - Added TaskStatus import for tests
- `crates/peekoo-plugin-host/src/registry.rs` - Added TaskStatus import for tests
- `crates/peekoo-plugin-store/src/lib.rs` - Added TaskStatus import for tests
- `apps/desktop-ui/src/types/task.ts` - Added agent labels
- `Cargo.toml` (workspace) - Added peekoo-mcp-server to members

## Testing the MCP Server

The MCP server is embedded in the main application. When a task is executed, it:
1. Finds an available TCP port (49152-65535 range)
2. Starts the MCP server on that port
3. Passes the port to `peekoo-agent-acp` via `PEEKOO_MCP_PORT` env var

To test the MCP server directly during development, you can create a test that:
1. Starts the MCP server on a known port
2. Connects an MCP client to test the tools