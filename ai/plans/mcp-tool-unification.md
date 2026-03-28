# Plan: MCP Tool Unification

## Overview

Unify all Peekoo-owned tools (task tools + plugin tools) behind the shared
`peekoo-mcp-server` HTTP server, and replace the two separate direct-injection
paths with a single shared RMCP streamable-HTTP client adapter that converts
MCP tools into `pi::tools::Tool` objects.

`pi` built-in tools (`read`, `bash`, `write`, etc.) are **not** moved — they
stay native in the agent runtime.

## Goals

- [x] Write plan
- [x] Phase 1 — Extract shared HTTP MCP client adapter into `peekoo-agent`
- [x] Phase 2 — Expand `peekoo-mcp-server` with all task tools
- [x] Phase 3 — Add plugin tool exposure to `peekoo-mcp-server`
- [x] Phase 4 — Switch `peekoo-agent-acp` to shared adapter
- [x] Phase 5 — Switch `peekoo-agent-app` chat/session path to MCP adapter
- [x] Phase 6 — Remove old direct task/plugin registration paths

## Design

### Transport

Streamable HTTP only. The shared MCP server already uses RMCP streamable HTTP
(`crates/peekoo-mcp-server/src/lib.rs`). Both agent entrypoints connect to it
over HTTP.

### Architecture

```
peekoo-mcp-server  (HTTP, shared, single process)
  ├── task tools   (task_create, task_list, task_update, task_delete,
  │                 task_toggle, task_assign, task_comment,
  │                 update_task_status, update_task_labels)
  └── plugin tools (plugin__{key}__{name} namespaced)

peekoo-agent  (new: mcp_client module)
  └── McpToolAdapter  rmcp Peer → pi::tools::Tool

peekoo-agent-acp  (task execution subprocess)
  └── uses McpToolAdapter (replaces local mcp_tools.rs adapter)

peekoo-agent-app  (app/chat sessions)
  └── uses McpToolAdapter (replaces extend_plugin_tools + register_native_tools)
```

### Dependency flow (unchanged)

`desktop-tauri` → `peekoo-agent-app` → `peekoo-agent` → `pi`
`peekoo-agent-acp` → `peekoo-agent`
`peekoo-mcp-server` → `peekoo-task-app`, `peekoo-plugin-host`

### Key design decisions

- **Plugin tool namespacing** — MCP tool names exposed as
  `plugin__{plugin_key}__{tool_name}` to avoid collisions with task tools.
- **Task scoping** — for scheduled task execution (ACP path), a thin
  client-side scoped wrapper injects `task_id` automatically for
  `task_comment`, `update_task_status`, `update_task_labels` so the agent
  prompt stays simple. For chat sessions, tools expose `task_id` explicitly.
- **`pi` built-ins** — remain native; not proxied through MCP.
- **No stdio MCP client** — HTTP only for this pass.

## Implementation Steps

### Phase 1 — Shared HTTP MCP client adapter in `peekoo-agent`

**Goal:** one reusable function that connects to an HTTP MCP server and returns
`Vec<Box<dyn pi::tools::Tool>>`.

Files to create/modify:
- `crates/peekoo-agent/src/mcp_client.rs` — new module
  - `pub async fn connect_http_mcp_tools(url: &str) -> anyhow::Result<(Vec<Box<dyn Tool>>, McpClientHandle)>`
  - `McpToolAdapter` struct implementing `pi::tools::Tool`
  - `McpClientHandle` for keeping the connection alive
- `crates/peekoo-agent/src/lib.rs` — `pub mod mcp_client;`
- `crates/peekoo-agent/Cargo.toml` — add `rmcp` (client + streamable-http-client-reqwest), `anyhow`, `rustls`

`McpToolAdapter` maps:
- `name()` → `tool.name`
- `description()` → `tool.description`
- `parameters()` → `tool.input_schema` as JSON
- `execute()` → `peer.call_tool(...)` → `ToolOutput`

Tests:
- Unit test: schema passthrough
- Integration test: connect to a real `start_tcp_server` and list/call tools

### Phase 2 — Expand `peekoo-mcp-server` with all task tools

**Goal:** expose all 7 task tools (currently only 3 are exposed).

Files to modify:
- `crates/peekoo-mcp-server/src/handler.rs` — add handlers for:
  - `task_create`
  - `task_list`
  - `task_update`
  - `task_delete`
  - `task_toggle`
  - `task_assign`
  (keep existing `task_comment`, `update_task_status`, `update_task_labels`)
- `crates/peekoo-mcp-server/src/lib.rs` — update test assertions

Tests:
- `list_tools` returns all 9 tool names
- Each new tool handler calls the correct `TaskService` method

### Phase 3 — Plugin tool exposure in `peekoo-mcp-server`

**Goal:** expose all loaded plugin tools through MCP with namespaced names.

Files to modify:
- `crates/peekoo-mcp-server/src/handler.rs` — add `PluginMcpHandler` or
  extend `TaskMcpHandler` to accept an optional `Arc<PluginRegistry>`
- `crates/peekoo-mcp-server/src/lib.rs` — update `start_tcp_server` signature
  to accept `Option<Arc<PluginRegistry>>`
- `crates/peekoo-mcp-server/Cargo.toml` — add `peekoo-plugin-host` dependency

Plugin tool routing:
- On `list_tools`: call `registry.all_tool_definitions()`, emit each as
  `plugin__{plugin_key}__{tool_name}`
- On `call_tool`: strip prefix, dispatch to `registry.call_tool(plugin_key, tool_name, args)`
- Use `spawn_blocking` for WASM calls (same as current `PluginToolAdapter`)

Tests:
- `list_tools` includes plugin tools with correct namespaced names
- `call_tool` dispatches to the correct plugin

### Phase 4 — Switch `peekoo-agent-acp` to shared adapter

**Goal:** replace the local `mcp_tools.rs` adapter with `peekoo_agent::mcp_client`.

Files to modify:
- `crates/peekoo-agent-acp/src/agent.rs` — replace
  `connect_task_mcp_tools(...)` call with `peekoo_agent::mcp_client::connect_http_mcp_tools(...)`
  plus task-scoped wrapper for `task_comment`, `update_task_status`,
  `update_task_labels`
- `crates/peekoo-agent-acp/src/mcp_tools.rs` — delete or reduce to only the
  task-scoped wrapper logic
- `crates/peekoo-agent-acp/Cargo.toml` — remove direct `rmcp` dep if no
  longer needed (it comes transitively through `peekoo-agent`)

Task-scoped wrapper: a thin `pi::tools::Tool` decorator that injects
`task_id` into the args before forwarding to the underlying `McpToolAdapter`.

Tests:
- ACP agent connects to MCP server and registers tools
- Task-scoped tools inject `task_id` correctly

### Phase 5 — Switch `peekoo-agent-app` chat/session to MCP adapter

**Goal:** replace `extend_plugin_tools` + `register_native_tools` with MCP
adapter in `create_agent_service`.

Files to modify:
- `crates/peekoo-agent-app/src/application.rs`
  - `create_agent_service()`: replace direct tool injection with
    `peekoo_agent::mcp_client::connect_http_mcp_tools(mcp_url)` and
    `service.register_native_tools(mcp_tools)`
  - Requires MCP server to be running before `create_agent_service` is called
    (already guaranteed by `start_plugin_runtime` ordering)
- `crates/peekoo-agent-app/src/mcp_server.rs` — expose `get_mcp_url()` helper

Tests:
- `create_agent_service` registers MCP-backed tools
- Chat session can call task and plugin tools via MCP

### Phase 6 — Remove old direct registration paths

**Goal:** delete dead code once both paths use MCP.

Files to delete/slim:
- `crates/peekoo-agent-app/src/task_tools.rs` — delete
- `crates/peekoo-agent-app/src/plugin_tool_impl.rs` — delete
- `crates/peekoo-agent/src/plugin_tool.rs` — delete
- `crates/peekoo-agent/src/lib.rs` — remove `plugin_tool` exports
- `crates/peekoo-agent/src/service.rs` — remove `extend_plugin_tools`,
  `register_native_tools` (or keep `register_native_tools` for future use)
- `crates/peekoo-agent-app/src/application.rs` — remove `plugin_tools` field,
  `PluginToolProviderImpl` usage, `call_plugin_tool` delegation
- `crates/peekoo-agent-app/Cargo.toml` — remove `rmcp` if no longer needed

## Files to Modify/Create

| File | Change |
|------|--------|
| `crates/peekoo-agent/src/mcp_client.rs` | **create** — shared HTTP MCP adapter |
| `crates/peekoo-agent/src/lib.rs` | add `pub mod mcp_client` |
| `crates/peekoo-agent/Cargo.toml` | add rmcp, anyhow, rustls |
| `crates/peekoo-mcp-server/src/handler.rs` | add 6 task tools + plugin handler |
| `crates/peekoo-mcp-server/src/lib.rs` | update signature + tests |
| `crates/peekoo-mcp-server/Cargo.toml` | add peekoo-plugin-host |
| `crates/peekoo-agent-acp/src/agent.rs` | use shared adapter |
| `crates/peekoo-agent-acp/src/mcp_tools.rs` | reduce to scoped wrapper only |
| `crates/peekoo-agent-app/src/application.rs` | use MCP adapter, remove direct injection |
| `crates/peekoo-agent-app/src/mcp_server.rs` | expose `get_mcp_url()` |
| `crates/peekoo-agent-app/src/task_tools.rs` | **delete** |
| `crates/peekoo-agent-app/src/plugin_tool_impl.rs` | **delete** |
| `crates/peekoo-agent/src/plugin_tool.rs` | **delete** |

## Testing Strategy

- Each phase has its own tests before moving to the next
- `just check` after each phase to catch compile errors early
- `just test` after phases 2, 3, 5 for integration coverage
- No regressions: chat sessions still have `pi` built-ins; task/plugin tools
  come from MCP

## Open Questions

- Should `register_native_tools` be kept on `AgentService` for future
  non-MCP tool injection? (Recommend: yes, keep it as a general escape hatch)
- Should plugin tools be exposed even when no plugins are loaded (empty list)?
  (Recommend: yes, just returns empty list — no error)
