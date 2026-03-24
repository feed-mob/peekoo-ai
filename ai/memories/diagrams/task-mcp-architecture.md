# Task MCP Architecture

This diagram shows how Peekoo's shared MCP server exposes task tools and how ACP bridges those tools into the real agent runtime.

**Related Code:**
- Shared MCP Startup: `crates/peekoo-agent-app/src/mcp_server.rs`
- MCP Server: `crates/peekoo-mcp-server/src/lib.rs`
- MCP Handler: `crates/peekoo-mcp-server/src/handler.rs`
- ACP MCP Bridge: `crates/peekoo-agent-acp/src/mcp_tools.rs`
- Task Runtime Service: `crates/peekoo-agent-app/src/task_runtime_service.rs`

```mermaid
flowchart TD
    subgraph App[Peekoo App Process]
        AppStart[AgentApplication::start_plugin_runtime]
        McpMgr[McpServerManager]
        RuntimeSvc[TaskRuntimeService]
        Product[ProductivityService]
        Notify[NotificationService]
    end

    subgraph MCP[Shared MCP Server]
        Http[Axum + StreamableHttpService<br/>http://127.0.0.1:PORT/mcp]
        Handler[TaskMcpHandler]
        Tool1[task_comment]
        Tool2[update_task_status]
        Tool3[update_task_labels]
    end

    subgraph ACP[ACP Subprocess]
        SessionCfg[session/new mcpServers]
        Bridge[McpToolAdapter bridge]
        Agent[peekoo-agent runtime]
    end

    AppStart --> McpMgr --> Http
    Http --> Handler
    Handler --> Tool1
    Handler --> Tool2
    Handler --> Tool3

    Tool1 --> RuntimeSvc
    Tool2 --> RuntimeSvc
    Tool3 --> RuntimeSvc
    RuntimeSvc --> Product
    RuntimeSvc --> Notify

    SessionCfg --> Bridge
    Bridge -->|list tools / call tools| Http
    Bridge --> Agent
```

## Notes

- The MCP server is shared across all task executions for the app lifetime.
- The server speaks RMCP streamable HTTP at `/mcp`.
- ACP receives MCP server definitions in `session/new`, connects to them, and re-exposes the MCP tools as native `pi` tools.
- `TaskRuntimeService` adds orchestration behavior on top of persistence, including follow-up mention requeueing and agent notifications.
- Only agent comments and agent status changes produce desktop notifications.
