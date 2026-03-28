# Peekoo MCP Server

MCP (Model Context Protocol) server that exposes Peekoo productivity tools for AI agents.

## Overview

This crate provides an MCP server that runs embedded in the Peekoo application and exposes task management, pomodoro timer, and app settings operations to AI agents via the Model Context Protocol.

**Architecture**: Single shared server started at app startup, serving all agent ACP processes and chat sessions.

## Endpoints

| Endpoint | Path | Description |
|----------|------|-------------|
| **Native Tools** | `/mcp` | All Peekoo-native tools: tasks, pomodoro, settings (24 tools) |
| **Plugin Tools** | `/mcp/plugins` | Third-party plugin tools via WASM runtime |

**Design Rationale**: Native Peekoo tools are unified at a single endpoint. Plugin tools remain separate as they require a different runtime (WASM) and are third-party extensions.

## Quick Start with mcporter

The easiest way to explore the MCP tools is using [mcporter](https://github.com/feed-mob/mcporter):

```bash
# Install mcporter if you haven't already
npm install -g mcporter

# Add the MCP server to your mcporter config (when Peekoo app is running)
mcporter config add peekoo-native http://127.0.0.1:49152/mcp --type http
mcporter config add peekoo-plugins http://127.0.0.1:49152/mcp/plugins --type http

# View tool documentation
mcporter list peekoo-native --schema    # View native tool docs
mcporter list peekoo-plugins --schema   # View plugin tool docs

# Call tools directly
mcporter call peekoo-native.pomodoro_status
mcporter call peekoo-native.task_list
```

**Config file**: [`mcporter.json`](./mcporter.json) - Example server definitions for reference.

**Note**: mcporter auto-discovers configs from `./config/mcporter.json` in the current directory. To use the provided config, run mcporter commands from this crate's directory, or copy `mcporter.json` to your project's `config/mcporter.json`.

## Native Tools at `/mcp`

### Task Tools (9)

| Tool | Description |
|------|-------------|
| `task_create` | Create a new task with title, priority, assignee, labels, description, scheduling, and recurrence rules. |
| `task_list` | List all tasks. Optionally filter by status (todo/in_progress/done). |
| `task_update` | Update a task's title, priority, status, assignee, labels, description, scheduling, or recurrence. |
| `task_delete` | Delete a task by its ID. |
| `task_toggle` | Toggle a task's completion status (todo <-> done). |
| `task_assign` | Assign a task to a user or agent. |
| `task_comment` | Add a comment to a task. Use this to ask questions or provide updates. |
| `update_task_labels` | Add or remove labels from a task. Use to mark state like `needs_clarification`, `agent_done`, `needs_review`. |
| `update_task_status` | Update task status. Use to mark as `in_progress`, `done`, `cancelled`. |

### Pomodoro Tools (10)

| Tool | Description |
|------|-------------|
| `pomodoro_status` | Get the current pomodoro timer status including mode, time remaining, and daily stats. |
| `pomodoro_start` | Start a new pomodoro session. Mode can be 'focus' or 'break'. |
| `pomodoro_pause` | Pause the currently active pomodoro timer. |
| `pomodoro_resume` | Resume a paused pomodoro timer. |
| `pomodoro_finish` | Finish or cancel the current pomodoro session. |
| `pomodoro_switch_mode` | Switch between focus and break modes. |
| `pomodoro_save_memo` | Save a memo for a pomodoro session. |
| `pomodoro_history` | Get pomodoro session history. Defaults to last 10 sessions. |
| `pomodoro_history_by_date_range` | Get pomodoro sessions within a date range (YYYY-MM-DD format). |
| `pomodoro_set_settings` | Configure pomodoro settings: work duration, break duration, long break settings. |

### Settings Tools (5)

| Tool | Description |
|------|-------------|
| `settings_get_active_sprite` | Get the currently active character (sprite) ID. |
| `settings_set_active_sprite` | Set the active character (sprite). Use `settings_list_sprites` to see available options. |
| `settings_list_sprites` | List all available characters (sprites) with their IDs and descriptions. |
| `settings_get_theme` | Get the current theme mode: 'light', 'dark', or 'system'. |
| `settings_set_theme` | Set the theme mode. Valid values: 'light', 'dark', 'system'. |

## Architecture

```
Main Application (AgentApplication)
  └─ MCP Server (http://127.0.0.1:PORT) [SHARED, starts at app startup]
      ├─ /mcp ───┬─ Task Tools (9)
      │           ├─ Pomodoro Tools (10) 
      │           └─ Settings Tools (5)
      └─ /mcp/plugins ── Plugin Tools (via WASM)
   
AgentScheduler
  ├─ Task 1: spawn peekoo-agent-acp ───┐
  ├─ Task 2: spawn peekoo-agent-acp ───┼── All connect via env vars
  └─ Task 3: spawn peekoo-agent-acp ───┘

Chat Sessions
  └─ Agent Service ──┬─ Connect to /mcp (native tools)
                     └─ Connect to /mcp/plugins (plugin tools)
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
🚀 [MCP] Starting server on http://127.0.0.1:49152/mcp
✅ [MCP] Server ready at http://127.0.0.1:49152/mcp
📋 [MCP] Available tools: 24 native tools (task, pomodoro, settings) (+ plugin tools if enabled)
```

When an agent connects to a task:

```
🔗 [MCP] Connecting agent to MCP server at http://127.0.0.1:49152/mcp
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
- `peekoo-task-domain` / `peekoo-task-app` - Task domain types, DTOs, and service interfaces
- `peekoo-pomodoro-app` - Pomodoro timer service and domain types
- `peekoo-app-settings` - App settings (sprites, themes) service
- `peekoo-plugin-host` - Plugin runtime (optional, behind `plugin-runtime` feature)
- `tokio` - Async runtime
- `schemars` - JSON Schema generation