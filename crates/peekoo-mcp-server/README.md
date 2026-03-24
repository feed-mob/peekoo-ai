# Peekoo MCP Server

MCP (Model Context Protocol) server that exposes task management tools for AI agents.

## Overview

This crate provides an MCP server that runs embedded in the Peekoo application and exposes task management operations to AI agents via the Model Context Protocol.

**Architecture**: Single shared server started at app startup, serving all agent ACP processes.

## Tools Provided

| Tool | Description |
|------|-------------|
| `task_comment` | Add a comment to a task. Use this to ask questions or provide updates. |
| `update_task_labels` | Add or remove labels from a task. Use to mark state like `needs_clarification`, `agent_done`, `needs_review`. |
| `update_task_status` | Update task status. Use to mark as `pending`, `in_progress`, `done`, `cancelled`. |

All tool calls require a `task_id` parameter to identify which task to operate on.

## Architecture

```
Main Application (AgentApplication)
  └─ MCP Server (tcp://127.0.0.1:PORT) [SHARED, starts at app startup]
      └─ TaskService (shared SQLite connection)
  
AgentScheduler
  ├─ Task 1: spawn peekoo-agent-acp ───┐
  ├─ Task 2: spawn peekoo-agent-acp ───┼── All connect via env vars
  └─ Task 3: spawn peekoo-agent-acp ───┘
```

## Startup Flow

1. **App startup** (`AgentApplication::new()`): Creates MCP server manager (not started yet)
2. **Runtime start** (`start_plugin_runtime()`): 
   - Starts MCP server on dynamic port
   - Logs server address: `🚀 [MCP] Starting server on tcp://127.0.0.1:PORT`
   - Passes address to AgentScheduler
3. **Task execution**: AgentScheduler passes `PEEKOO_MCP_HOST` and `PEEKOO_MCP_PORT` env vars to each agent subprocess

## Tool Schemas

#### task_comment

```json
{
  "name": "task_comment",
  "description": "Add a comment to a task. Use this to ask questions or provide updates.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "task_id": { "type": "string", "description": "Task ID to comment on" },
      "text": { "type": "string", "description": "Comment text (supports markdown)" }
    },
    "required": ["task_id", "text"]
  }
}
```

#### update_task_labels

```json
{
  "name": "update_task_labels",
  "description": "Add or remove labels from a task.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "task_id": { "type": "string", "description": "Task ID" },
      "add_labels": { "type": "array", "items": { "type": "string" }, "description": "Labels to add" },
      "remove_labels": { "type": "array", "items": { "type": "string" }, "description": "Labels to remove" }
    },
    "required": ["task_id"]
  }
}
```

#### update_task_status

```json
{
  "name": "update_task_status",
  "description": "Update task status.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "task_id": { "type": "string", "description": "Task ID" },
      "status": { "type": "string", "enum": ["pending", "in_progress", "done", "cancelled"], "description": "New status" }
    },
    "required": ["task_id", "status"]
  }
}
```

## Logs

When the app starts, you'll see:

```
🚀 [MCP] Starting server on tcp://127.0.0.1:49152
✅ [MCP] Server ready at tcp://127.0.0.1:49152
📋 [MCP] Available tools: task_comment, update_task_labels, update_task_status
✅ [MCP] Server initialized at tcp://127.0.0.1:49152 (shared)
🔗 [MCP] Scheduler configured with server at tcp://127.0.0.1:49152
```

When a task is executed:

```
🔗 [MCP] Using shared server at tcp://127.0.0.1:49152 for task {task_id}
🔗 [MCP] Connecting agent to MCP server at tcp://127.0.0.1:49152
```

## Agent Environment Variables

Agents receive MCP server configuration via environment variables:

- `PEEKOO_MCP_HOST` - MCP server host (e.g., `127.0.0.1`)
- `PEEKOO_MCP_PORT` - MCP server port (e.g., `49152`)

## Labels

The following labels are used by agents:

| Label | When Applied | Meaning |
|-------|--------------|---------|
| `agent_working` | Agent starts task | Agent is actively working |
| `needs_clarification` | Agent has questions | Agent needs user input |
| `agent_done` | Agent completes task | Agent finished work |
| `needs_review` | Agent completes task | User should review results |
| `agent_failed` | Agent fails after retries | Agent could not complete |

## Dependencies

- `rmcp` - Official Rust MCP SDK from [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)
- `peekoo-productivity-domain` - Task service trait and types
- `tokio` - Async runtime
- `schemars` - JSON Schema generation