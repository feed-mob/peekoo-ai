# 2026-03-31 feat: peekoo-agent-skill with mcporter and managed Node PATH

## Problem
ACP agents that don't support MCP natively (like pi-acp) have no way to access peekoo's productivity tools (tasks, pomodoro, calendar, plugins). Additionally, users without system Node.js installed can't run `npx` commands.

## Solution

### 1. peekoo-agent-skill
Added a bundled skill that teaches ACP agents how to use mcporter to call peekoo MCP tools:
- `SKILL.md` — Instructions for discovering and calling tools via `npx mcporter call`
- `mcporter.json` — Config file with MCP server URLs (port updated at runtime)

### 2. Managed Node.js in PATH
Prepended peekoo's managed Node.js `bin/` directory to the agent's PATH in `build_launch_env()`. This ensures `npx` commands work even when the user doesn't have Node.js installed on their system.

### 3. Runtime mcporter Config
After the MCP server binds to a dynamic port, `write_mcporter_config()` updates the skill's `mcporter.json` with the actual port. This ensures mcporter always connects to the correct endpoint.

## Files Changed
- `crates/peekoo-agent-app/src/runtime_adapters/mod.rs` — prepend managed node bin to PATH
- `crates/peekoo-agent-app/src/mcp_server.rs` — write mcporter config after port binding, pass workspace_dir through call chain
- `templates/persona/.agents/skills/peekoo-agent-skill/SKILL.md` — create
- `templates/persona/.agents/skills/peekoo-agent-skill/mcporter.json` — create

## Architecture
```
ACP Agent → reads SKILL.md → runs npx mcporter call ...
  ↓
mcporter (via managed Node.js in PATH)
  ↓ reads mcporter.json with actual port
  ↓ connects to http://127.0.0.1:<port>/mcp
  ↓
Peekoo MCP Server
```
