# ACP Task Execution Architecture

This diagram shows how Peekoo uses ACP (Agent Client Protocol) to launch `peekoo-agent-acp` as a subprocess and drive scheduled task execution.

**Related Code:**
- Scheduler: `crates/peekoo-agent-app/src/agent_scheduler.rs`
- ACP Agent: `crates/peekoo-agent-acp/src/agent.rs`
- Task Context: `crates/peekoo-agent-acp/src/context.rs`
- Agent Runtime: `crates/peekoo-agent/src/service.rs`

```mermaid
sequenceDiagram
    participant Scheduler as AgentScheduler
    participant ACP as peekoo-agent-acp
    participant ACPConn as ACP Connection
    participant Session as ACP Session State
    participant Agent as peekoo-agent::AgentService

    Scheduler->>ACP: spawn subprocess (stdio)
    Scheduler->>ACPConn: initialize(protocol=v1)
    ACPConn-->>Scheduler: initializeResponse(agentInfo, mcpCapabilities)

    Scheduler->>ACPConn: session/new(cwd, mcpServers)
    ACPConn->>Session: store cwd + mcpServers
    ACPConn-->>Scheduler: sessionId

    Scheduler->>ACPConn: prompt(sessionId, TaskContext JSON)
    ACPConn->>Session: load session state
    ACPConn->>ACP: deserialize TaskContext
    ACP->>Agent: build AgentService for task
    ACP->>Agent: prompt(TaskContext::to_prompt())

    loop Agent events
        Agent-->>ACP: AgentEvent
        ACP-->>Scheduler: session/update(message chunk)
    end

    Agent-->>ACP: final assistant text
    ACP-->>Scheduler: promptResponse(stopReason=EndTurn)
    Scheduler->>ACP: kill subprocess
```

## Notes

- ACP is the transport boundary between the scheduler and the actual agent runtime.
- `session/new` carries MCP server configuration, so task tools are attached in an ACP-native way.
- The scheduler no longer fabricates task comments; it only orchestrates execution and listens for ACP updates.
- Each task run can reuse a task-scoped persistent agent session when one exists.
