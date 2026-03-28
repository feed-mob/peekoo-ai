## 2026-03-28 09:00: refactor: MCP tool unification

**What changed:**
- Created `crates/peekoo-agent/src/mcp_client.rs` — shared HTTP MCP client adapter (`connect_http_mcp_tools`, `McpToolAdapter`, `McpClientHandle`)
- Expanded `crates/peekoo-mcp-server/src/handler.rs` to expose all 9 task tools via `#[tool_router]` macros
- Created `crates/peekoo-mcp-server/src/plugin.rs` — `PluginMcpHandler` serving plugin tools at `/mcp/plugins` with `plugin__{key}__{name}` namespacing, behind `plugin-runtime` feature flag
- Rewrote `crates/peekoo-agent-acp/src/mcp_tools.rs` — now only contains `TaskScopedTool` wrapper (injects `task_id` for scoped tools) and `summarize_agent_event`; removed old local `McpToolAdapter`
- Updated `crates/peekoo-agent-acp/src/agent.rs` to use `peekoo_agent::mcp_client::connect_http_mcp_tools`
- Updated `crates/peekoo-agent-app/src/application.rs` `create_agent_service` to connect to the shared MCP server instead of directly injecting task/plugin tools
- Added `store_mcp_handle()` and `_mcp_handles` field to `AgentService` to keep connections alive
- Deleted `crates/peekoo-agent-app/src/task_tools.rs` (replaced by MCP server handler)
- Removed `pub use plugin_tool::{PluginToolAdapter, PluginToolProvider, PluginToolSpec}` re-export from `peekoo-agent/src/lib.rs`

**Why:**
- Two separate direct-injection paths (ACP agent and chat session) were duplicating tool registration logic
- Centralising all Peekoo-owned tools behind the shared MCP server makes it easier to add/remove tools in one place
- `pi` built-in tools (read, bash, write, etc.) remain native and are not proxied

**Notes:**
- `plugin_tools: Arc<PluginToolProviderImpl>` and `call_plugin_tool` are intentionally kept on `AgentApplication` — still used by the Tauri `plugin_call_tool` command for frontend-initiated plugin calls
- Plugin tools are served at `/mcp/plugins` (separate endpoint from task tools at `/mcp`)
- `TaskScopedTool` wraps tools that need automatic `task_id` injection: `task_comment`, `update_task_status`, `update_task_labels`

**Files affected:**
- `crates/peekoo-agent/src/mcp_client.rs` (created)
- `crates/peekoo-agent/src/lib.rs`
- `crates/peekoo-agent/src/service.rs`
- `crates/peekoo-agent/Cargo.toml`
- `crates/peekoo-mcp-server/src/handler.rs`
- `crates/peekoo-mcp-server/src/plugin.rs` (created)
- `crates/peekoo-mcp-server/src/lib.rs`
- `crates/peekoo-mcp-server/Cargo.toml`
- `crates/peekoo-agent-acp/src/mcp_tools.rs`
- `crates/peekoo-agent-acp/src/agent.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/mcp_server.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-agent-app/src/task_tools.rs` (deleted)
- `crates/peekoo-agent-app/Cargo.toml`
