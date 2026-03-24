# 2026-03-24 refactor: Shared MCP server for all agent tasks

## What changed

- Created shared MCP server that starts once at app startup and serves all agents
- Removed per-task MCP server creation (no longer starts a new server for each task)
- MCP server runs on a **dedicated thread with its own persistent tokio runtime**
- MCP server starts **eagerly** in `start_plugin_runtime()` (not lazily)
- All agent ACP processes connect to the same MCP server via env vars
- Removed `mcp_server_port` and `mcp_server_host` from TaskContext
- Agents now read MCP configuration from environment variables only
- Fixed port binding race condition: listener is now kept alive between bind and server start
- Fixed critical runtime drop issue: MCP server task was dying when `block_on` finished

## Why

Better architecture with a single shared MCP server:
- Simpler lifecycle management (one server for app lifetime)
- Better resource usage (no port allocation per task)
- No race conditions (server starts before any agents)
- Consistent address (all agents connect to same endpoint)
- MCP server available immediately at app startup for manual testing

### Critical bug fix: MCP server dying with block_on

The original implementation had a critical bug:
1. `AgentScheduler` spawns a thread with `tokio::runtime::Builder::new_current_thread()`
2. Inside `rt.block_on()`, it called `ensure_mcp_server_started()` which spawned the MCP server via `tokio::spawn`
3. The MCP server task ran on that temporary runtime
4. When `block_on` finished, the runtime was dropped
5. **The MCP server task died with it**

The fix: `start_sync()` spawns a **dedicated thread** for the MCP server, with its own persistent runtime that stays alive until the cancellation token is triggered.

## Architecture

```
Main Application (AgentApplication)
  └─ start_plugin_runtime()
      ├─ McpServerManager::start_sync() → spawns dedicated MCP thread
      │   └─ MCP Thread (persistent runtime)
      │       └─ McpServerManager (tcp://127.0.0.1:PORT)
      │           └─ TaskService (shared SQLite connection)
      │       └─ Cancelled via shutdown_token on app shutdown
      └─ AgentScheduler::start()
          └─ Worker threads
              └─ check_and_execute_tasks()
                  └─ get_mcp_address() from global state
                  └─ spawn peekoo-agent-acp subprocess
                      └─ connects to MCP via PEEKOO_MCP_HOST/PORT env vars
```

## Files affected

- `crates/peekoo-agent-app/src/mcp_server.rs` - Added: `start_sync()`, `get_mcp_address()`, `shutdown()`, global `MCP_SERVER_STATE`
- `crates/peekoo-agent-app/src/application.rs` - Call `mcp_server::start_sync()` in `start_plugin_runtime()`
- `crates/peekoo-agent-app/src/agent_scheduler.rs` - Removed: duplicated MCP state and startup logic; use `get_mcp_address()`
- `crates/peekoo-agent-acp/src/context.rs` - Removed MCP fields from TaskContext
- `crates/peekoo-agent-acp/src/agent.rs` - Read MCP config from env vars only

## Logs

```
🚀 [MCP] Starting server on dedicated thread...
🚀 [MCP] Binding server on tcp://127.0.0.1:49152
✅ [MCP] Server listening at tcp://127.0.0.1:49152
✅ [MCP] Server ready at tcp://127.0.0.1:49152
🔗 [MCP] Using shared server at tcp://127.0.0.1:49152 for task {task_id}
```