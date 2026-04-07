# Plan: peekoo-agent-skill with Managed Node PATH

## Overview
Add a bundled skill that gives ACP agents (that don't support MCP natively) access to peekoo's productivity tools via mcporter CLI. Also ensure `npx` works for all users by prepending peekoo's managed Node.js bin directory to the agent's PATH.

## Goals
- [x] ACP agents can discover and call peekoo MCP tools via mcporter
- [x] `npx mcporter call ...` works regardless of whether user has system Node.js
- [x] mcporter.json config is auto-generated with the actual MCP server port
- [x] Skill is auto-synced to workspace on app start

## Design

### Architecture Flow
```
ACP Agent (e.g., pi-acp)
    ↓ reads SKILL.md via skill tool
    ↓ runs: npx mcporter list peekoo-native --config <workspace>/.agents/skills/peekoo-agent-skill/mcporter.json
    ↓
mcporter (via npx, managed Node.js in PATH)
    ↓ reads mcporter.json with actual port
    ↓ connects to http://127.0.0.1:<port>/mcp
    ↓
Peekoo MCP Server
```

### Components
1. **Managed Node PATH** — Prepend `~/.peekoo/data/resources/node/<version>/bin/` to agent's PATH in `build_launch_env()`
2. **Skill Template** — `peekoo-agent-skill/SKILL.md` with usage instructions
3. **mcporter.json Template** — Placeholder config with port 49152
4. **Runtime Config Update** — `mcp_server.rs` writes actual port to mcporter.json after binding

## Implementation Steps

1. **Add managed node bin to PATH** in `runtime_adapters/mod.rs`
   - After forwarding PATH from parent, prepend managed node bin directory
   - Use `peekoo_node_runtime::NodeRuntime::instance()` to resolve the path

2. **Create skill template files**
   - `templates/persona/.agents/skills/peekoo-agent-skill/SKILL.md`
   - `templates/persona/.agents/skills/peekoo-agent-skill/mcporter.json`

3. **Update mcp_server.rs** to write mcporter config
   - After port binding, write `<workspace>/.agents/skills/peekoo-agent-skill/mcporter.json`
   - Need to pass workspace directory to the MCP server startup

4. **What happens automatically**
   - `build.rs` auto-discovers both files in the skill folder
   - `sync_skill_templates()` copies to workspace on app start
   - MCP server overwrites mcporter.json with actual port

## Files to Modify/Create
- `crates/peekoo-agent-app/src/runtime_adapters/mod.rs` — prepend managed node bin to PATH
- `crates/peekoo-agent-app/src/mcp_server.rs` — write mcporter config after port binding
- `templates/persona/.agents/skills/peekoo-agent-skill/SKILL.md` — create
- `templates/persona/.agents/skills/peekoo-agent-skill/mcporter.json` — create

## Testing Strategy
- Unit test for PATH construction with managed node
- Integration test: skill template is auto-discovered by build.rs
- Manual test: agent can run `npx mcporter list` after app starts

## Open Questions
- How does mcp_server.rs get access to the workspace directory? Need to check current code.
- Should we handle the case where managed node download fails gracefully?
