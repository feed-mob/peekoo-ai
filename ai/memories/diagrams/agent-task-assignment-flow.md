# Agent Task Assignment Flow

This diagram shows how assigning a task to `peekoo-agent` causes the scheduler to pick it up and how later `@peekoo-agent` comments trigger follow-up work.

**Related Code:**
- Task Assignment and State: `crates/peekoo-agent-app/src/productivity.rs`
- Follow-up Runtime Logic: `crates/peekoo-agent-app/src/task_runtime_service.rs`
- Scheduler: `crates/peekoo-agent-app/src/agent_scheduler.rs`
- ACP Agent: `crates/peekoo-agent-acp/src/agent.rs`

```mermaid
sequenceDiagram
    participant User
    participant UI as Tasks UI
    participant App as AgentApplication
    participant Product as ProductivityService
    participant Runtime as TaskRuntimeService
    participant Scheduler as AgentScheduler
    participant ACP as peekoo-agent-acp
    participant MCP as Shared MCP Server

    Note over User,MCP: Initial Assignment
    User->>UI: Assign task to peekoo-agent
    UI->>App: update_task(assignee="peekoo-agent")
    App->>Product: update task
    Product->>Product: set agent_work_status="pending"

    Scheduler->>Product: list_tasks_for_agent_execution()
    Product-->>Scheduler: pending agent task
    Scheduler->>Product: claim_task_for_agent()
    Product->>Product: set agent_work_status="claimed"

    Scheduler->>ACP: spawn + ACP session
    ACP->>MCP: connect task tools
    ACP->>Product: load comments/activity for TaskContext
    ACP->>MCP: task_comment / update_task_status / update_task_labels
    MCP->>Runtime: apply task updates
    Runtime->>Product: persist task changes

    Note over User,MCP: Follow-up Comment
    User->>UI: add comment "@peekoo-agent ..."
    UI->>App: add_task_comment()
    App->>Runtime: add_task_comment()
    Runtime->>Product: persist comment
    Runtime->>Product: requeue_agent_task() => pending
    Runtime->>Scheduler: trigger_now()

    Scheduler->>Product: list_tasks_for_agent_execution()
    Product-->>Scheduler: requeued task
    Scheduler->>ACP: start follow-up run
    ACP->>ACP: reuse task session if available
    ACP->>MCP: comment/status updates for follow-up response
```

## Notes

- Agent-assigned tasks become runnable by setting internal `agent_work_status` to `pending`.
- The scheduler picks tasks by internal agent-work state, not only by visible task status like `todo` or `done`.
- Follow-up comments with `@peekoo-agent` immediately requeue the task and trigger the scheduler without waiting for the next poll tick.
- Follow-up runs now receive comment-only context in chronological order, with the latest comment explicitly highlighted.
