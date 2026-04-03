# Peekoo Agent ACP

ACP transport wrapper for [`peekoo-agent`](../peekoo-agent).

## Overview

`peekoo-agent-acp` exposes `peekoo-agent` over the
[Agent Client Protocol](https://github.com/modelcontextprotocol/agent-client-protocol).
It is intentionally thin:

- accepts ACP `initialize`, `new_session`, `load_session`, and `prompt` requests
- builds a `peekoo-agent::service::AgentService`
- forwards ACP session MCP servers into `peekoo-agent`
- streams basic session updates back to the ACP client

This crate should not grow its own independent agent feature set. If
`peekoo-agent` gains runtime capabilities, this ACP wrapper should proxy them
instead of reimplementing them.

## Architecture

```text
ACP client
  -> peekoo-agent-acp
      -> peekoo-agent
          -> configured ACP runtime (OpenCode, Codex, Claude Code, etc.)
```

For scheduled task execution, the flow is:

```text
AgentScheduler
  -> starts peekoo-agent-acp
  -> injects task MCP servers into ACP new_session(...)
  -> peekoo-agent-acp forwards those MCP servers into AgentServiceConfig
  -> peekoo-agent forwards them to the downstream ACP backend
  -> runtime can call Peekoo task MCP tools
```

## MCP Behavior

The active MCP integration path is ACP session `mcp_servers`.

- `new_session` and `load_session` store the incoming ACP session context
- `build_agent_service(...)` copies `session_context.mcp_servers` into
  `AgentServiceConfig.mcp_servers`
- `peekoo-agent` passes those MCP servers to its backend
- the downstream ACP runtime receives the same session MCP server definitions

This is the path used by the task scheduler. It lets future ACP runtimes work
the same way as long as they honor ACP session MCP servers.

## Task Scheduler Integration

The scheduler expects scheduled runs to produce task-side effects through MCP
tools, not just plain text output.

The relevant task tools are exposed by [`peekoo-mcp-server`](../peekoo-mcp-server):

- `task_comment`
- `update_task_status`
- `update_task_labels`

`peekoo-agent-acp` does not implement those tools itself. It only proxies the
MCP server definitions through to `peekoo-agent` and the configured ACP runtime.

## Runtime Selection

Internal subprocesses still launch `peekoo-agent-acp`, but runtime selection is
owned by `peekoo-agent` configuration.

Today that means:

- `peekoo-agent-acp` is the stable ACP-facing entrypoint
- `peekoo-agent` chooses the provider/runtime
- the default provider is currently OpenCode

## Development Notes

- Keep this crate transport-focused.
- Do not reintroduce a second MCP tool path here.
- If a feature seems to require custom wrapper logic, prefer first checking
  whether it should instead be implemented in `peekoo-agent` and proxied.

## Verification

Useful checks when changing this crate:

```bash
cargo test -p peekoo-agent-acp -- --nocapture
cargo check -p peekoo-agent-acp -p peekoo-agent -p peekoo-agent-app
```
