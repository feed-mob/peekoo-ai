# peekoo-agent-skill / mcporter Architecture

## Overview
ACP agents that don't support MCP natively can access peekoo productivity tools via the mcporter CLI. The skill is auto-synced to the workspace, and the mcporter config is updated at runtime with the actual MCP server port.

## Architecture

```mermaid
graph TD
    A[App Start] --> B[ensure_agent_workspace]
    B --> C[sync_skill_templates]
    C --> D[Write SKILL.md + mcporter.json to workspace]
    
    E[MCP Server Start] --> F[Bind to dynamic port]
    F --> G[Write actual port to mcporter.json]
    
    H[ACP Agent Spawn] --> I[PATH includes managed node bin]
    I --> J[Agent reads SKILL.md via skill tool]
    J --> K[Agent runs npx mcporter call ...]
    K --> L[mcporter reads mcporter.json]
    L --> M[Connects to http://127.0.0.1:port/mcp]
    M --> N[Peekoo MCP Server]
    
    D -.-> L
    G -.-> L
```

## Data Flow

```mermaid
sequenceDiagram
    participant App as Peekoo App
    participant WS as Workspace
    participant MCP as MCP Server
    participant Agent as ACP Agent
    participant MC as mcporter

    App->>WS: sync_skill_templates()
    Note over App,WS: Writes SKILL.md + mcporter.json (port 49152)
    
    App->>MCP: Start MCP server
    MCP->>MCP: Bind to dynamic port
    MCP->>WS: Write actual port to mcporter.json
    
    App->>Agent: Spawn with PATH (managed node bin prepended)
    Agent->>Agent: Read SKILL.md via skill tool
    Agent->>MC: npx mcporter list peekoo-native --config mcporter.json
    MC->>WS: Read mcporter.json (actual port)
    MC->>MCP: Connect to http://127.0.0.1:port/mcp
    MCP-->>MC: Return tool list
    MC-->>Agent: Available tools
    
    Agent->>MC: npx mcporter call peekoo-native.pomodoro_status
    MC->>MCP: Call tool via HTTP
    MCP-->>MC: Tool result
    MC-->>Agent: Response
```

## Key Components

| Component | Location | Purpose |
|-----------|----------|---------|
| Skill Template | `templates/persona/.agents/skills/peekoo-agent-skill/` | Bundled skill files |
| build.rs | `crates/peekoo-agent-app/build.rs` | Auto-discovers skill files |
| sync_skill_templates() | `workspace_bootstrap.rs` | Copies to workspace on app start |
| write_mcporter_config() | `mcp_server.rs` | Updates port in mcporter.json |
| build_launch_env() | `runtime_adapters/mod.rs` | Prepends managed node bin to PATH |

## Port Resolution

The MCP server binds to a dynamic port (scans 49152-65535). The mcporter.json template starts with port 49152, but is overwritten at runtime with the actual bound port. This ensures mcporter always connects to the correct endpoint.

## PATH Resolution

```
Agent PATH = <managed_node_bin>:<system_PATH>
```

The managed Node.js bin directory is prepended to ensure `npx` is always available, even if the user doesn't have system Node.js installed. Resolution order:
1. Managed runtime: `~/.peekoo/data/resources/node/v20.18.0/bin/`
2. System PATH: inherited from parent process
