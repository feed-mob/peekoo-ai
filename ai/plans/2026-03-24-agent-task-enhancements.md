# Implementation Plan: Enhanced Agent Task Execution (Real Agent + Embedded MCP)

## Goal
Make the ACP agent actually work with LLM + MCP tools, using embedded MCP server in main app.

## Architecture

```
Main App Process (peekoo-agent-app)
    ├─ AgentScheduler
    │   ├─ Starts embedded MCP Server (in-process, rmcp)
    │   │   └─ Direct access to TaskService
    │   │   └─ Tools: task_comment, update_task_labels, update_task_status
    │   └─ Spawns peekoo-agent-acp subprocess
    │       └─ ACP protocol over stdio
    │           └─ Uses AgentService (peekoo-agent) with real LLM
    │               └─ LLM calls tools via MCP over stdio
    │                   └─ MCP client in subprocess → MCP server in main app
    └─ NotificationService

Key: MCP Server runs in main app (not subprocess), giving direct TaskService access
```

## Implementation Steps

### Phase 1: Create MCP Server Crate (Embedded)

#### 1.1 Create new crate: `peekoo-mcp-server`
**Location**: `crates/peekoo-mcp-server/`

**Purpose**: In-process MCP server with direct TaskService access

**Cargo.toml**:
```toml
[package]
name = "peekoo-mcp-server"
version = "0.1.0"
edition = "2024"

[dependencies]
# Official Rust MCP SDK from modelcontextprotocol/rust-sdk
rmcp = { git = "https://github.com/modelcontextprotocol/rust-sdk", branch = "main", features = ["server"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"
async-trait = "0.1"
peekoo-productivity-domain = { path = "../peekoo-productivity-domain" }

[lib]
name = "peekoo_mcp_server"
```

#### 1.2 Implement MCP Server Handler
**File**: `crates/peekoo-mcp-server/src/lib.rs`

```rust
use rmcp::{
    handler::server::ServerHandler,
    model::*,
    tool,
    ErrorData as McpError,
};
use std::sync::Arc;
use peekoo_productivity_domain::task::TaskService;

pub struct TaskMcpHandler {
    task_service: Arc<dyn TaskService>,
}

impl TaskMcpHandler {
    pub fn new(task_service: Arc<dyn TaskService>) -> Self {
        Self { task_service }
    }
}

#[tool]
impl TaskMcpHandler {
    #[tool(
        name = "task_comment",
        description = "Add a comment to a task. Use this to ask questions or provide updates."
    )]
    async fn task_comment(
        &self,
        #[tool(param(description = "Task ID to comment on"))]
        task_id: String,
        #[tool(param(description = "Comment text (supports markdown)"))]
        text: String,
    ) -> Result<CallToolResult, McpError> {
        match self.task_service.add_task_comment(&task_id, &text, "agent") {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text("Comment added successfully")])),
            Err(e) => Ok(CallToolResult::error(e)),
        }
    }

    #[tool(
        name = "update_task_labels",
        description = "Add or remove labels from a task. Use to mark state like 'needs_clarification', 'agent_done', 'needs_review'."
    )]
    async fn update_task_labels(
        &self,
        #[tool(param(description = "Task ID"))]
        task_id: String,
        #[tool(param(description = "Labels to add"))]
        add_labels: Option<Vec<String>>,
        #[tool(param(description = "Labels to remove"))]
        remove_labels: Option<Vec<String>>,
    ) -> Result<CallToolResult, McpError> {
        // Add labels
        if let Some(labels) = add_labels {
            for label in labels {
                if let Err(e) = self.task_service.add_task_label(&task_id, &label) {
                    return Ok(CallToolResult::error(e));
                }
            }
        }
        
        // Remove labels
        if let Some(labels) = remove_labels {
            for label in labels {
                if let Err(e) = self.task_service.remove_task_label(&task_id, &label) {
                    return Ok(CallToolResult::error(e));
                }
            }
        }
        
        Ok(CallToolResult::success(vec![Content::text("Labels updated")]))
    }

    #[tool(
        name = "update_task_status",
        description = "Update task status. Use to mark as 'in_progress', 'done', 'cancelled'."
    )]
    async fn update_task_status(
        &self,
        #[tool(param(description = "Task ID"))]
        task_id: String,
        #[tool(param(description = "New status: pending, in_progress, done, cancelled"))]
        status: String,
    ) -> Result<CallToolResult, McpError> {
        let task_status = match status.as_str() {
            "pending" => TaskStatus::Pending,
            "in_progress" => TaskStatus::InProgress,
            "done" => TaskStatus::Done,
            "cancelled" => TaskStatus::Cancelled,
            _ => return Ok(CallToolResult::error(format!("Invalid status: {}", status))),
        };
        
        match self.task_service.update_task_status(&task_id, task_status) {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text("Status updated")])),
            Err(e) => Ok(CallToolResult::error(e)),
        }
    }
}

#[async_trait::async_trait]
impl ServerHandler for TaskMcpHandler {
    async fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "peekoo-task-tools".to_string(),
            version: "0.1.0".to_string(),
        }
    }
}

/// Start MCP server with stdio transport
pub async fn start_mcp_server(task_service: Arc<dyn TaskService>) -> Result<(), Box<dyn std::error::Error>> {
    let handler = TaskMcpHandler::new(task_service);
    let transport = rmcp::transport::StdioServerTransport::new();
    
    handler.serve(transport).await?;
    Ok(())
}

### Phase 2: Make ACP Agent Actually Work (Real LLM + MCP)

#### 2.1 Update peekoo-agent-acp to use AgentService
**File**: `crates/peekoo-agent-acp/Cargo.toml`

Add dependencies:
```toml
[dependencies]
# ... existing deps ...
peekoo-agent = { path = "../peekoo-agent" }
rmcp = { version = "0.16", features = ["client"] }
```

#### 2.2 Create MCP Tool Adapter
**File**: `crates/peekoo-agent-acp/src/mcp_tools.rs`

```rust
//! MCP tool adapter - exposes MCP tools as pi::tools::Tool for the agent

use pi::tools::{Tool, ToolOutput};
use rmcp::{
    model::{CallToolRequestParam, Content},
    service::ServiceExt,
    transport::StdioClientTransport,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Adapter that wraps MCP client as pi Tool
pub struct McpToolAdapter {
    client: Arc<Mutex<rmcp::Client>>,
    tool_name: String,
    tool_schema: Value,
}

#[async_trait::async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        self.tool_schema
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("MCP tool")
    }

    fn parameters(&self) -> Value {
        self.tool_schema
            .get("inputSchema")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({"type": "object"}))
    }

    async fn execute(&self, params: Value) -> anyhow::Result<ToolOutput> {
        let client = self.client.lock().await;
        
        let request = CallToolRequestParam {
            name: self.tool_name.clone(),
            arguments: Some(params),
        };
        
        let result = client.call_tool(request).await?;
        
        // Convert MCP result to pi ToolOutput
        let text = result.content.iter()
            .filter_map(|c| match c {
                Content::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        Ok(ToolOutput::Text(text))
    }
}

/// Create pi tools from MCP server
pub async fn create_tools_from_mcp(
    mcp_stdin: std::process::ChildStdin,
    mcp_stdout: std::process::ChildStdout,
) -> anyhow::Result<Vec<Box<dyn Tool>>> {
    let transport = StdioClientTransport::from_stdio(mcp_stdin, mcp_stdout);
    let client = ().serve(transport).await?;
    
    // List available tools
    let tools_result = client.list_tools(Default::default()).await?;
    
    let mut tools: Vec<Box<dyn Tool>> = Vec::new();
    let client = Arc::new(Mutex::new(client));
    
    for tool in tools_result.tools {
        let schema = serde_json::to_value(&tool)?;
        tools.push(Box::new(McpToolAdapter {
            client: client.clone(),
            tool_name: tool.name,
            tool_schema: schema,
        }));
    }
    
    Ok(tools)
}
```

#### 2.3 Update ACP Agent to Use Real LLM
**File**: `crates/peekoo-agent-acp/src/agent.rs`

```rust
use peekoo_agent::{
    config::AgentServiceConfig,
    service::AgentService,
};
use crate::mcp_tools::create_tools_from_mcp;
use crate::context::TaskContext;

pub struct PeekooAgent {
    session_update_tx: mpsc::UnboundedSender<(SessionNotification, oneshot::Sender<()>)>,
    next_session_id: Cell<u64>,
    // Store MCP child process handles
    mcp_processes: RefCell<Vec<std::process::Child>>,
}

#[async_trait(?Send)]
implement acp::Agent for PeekooAgent {
    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        // Parse task context from prompt
        let task_context: TaskContext = parse_task_context(&arguments)?;
        
        // Create agent config
        let config = AgentServiceConfig {
            provider: Some("anthropic".to_string()),
            model: Some("claude-3-7-sonnet-latest".to_string()),
            system_prompt: Some(build_system_prompt(&task_context)),
            ..Default::default()
        };
        
        // Initialize agent
        let mut agent = AgentService::new(config).await
            .map_err(|e| acp::Error::other(e))?;
        
        // Connect to MCP servers and add tools
        if let Some(mcp_servers) = arguments.mcp_servers {
            for server_config in mcp_servers {
                // Spawn MCP server subprocess
                let mut child = spawn_mcp_server(&server_config).await?;
                
                // Get stdio handles
                let stdin = child.stdin.take().ok_or_else(|| 
                    acp::Error::other("Failed to get MCP stdin")
                )?;
                let stdout = child.stdout.take().ok_or_else(|| 
                    acp::Error::other("Failed to get MCP stdout")
                )?;
                
                // Create tools from MCP
                let tools = create_tools_from_mcp(stdin, stdout).await
                    .map_err(|e| acp::Error::other(e))?;
                
                // Register tools with agent
                agent.register_native_tools(tools);
                
                // Store child process
                self.mcp_processes.borrow_mut().push(child);
            }
        }
        
        // Stream events back to scheduler
        let task_id = task_context.task_id.clone();
        let (tx, mut rx) = mpsc::channel(100);
        
        let prompt_text = task_context.to_prompt();
        
        // Run agent in background
        tokio::spawn(async move {
            let result = agent.prompt(&prompt_text, |event| {
                // Send events through channel
                let _ = tx.try_send(event);
            }).await;
            
            result
        });
        
        // Forward events to scheduler via session notifications
        while let Some(event) = rx.recv().await {
            let notification = convert_event_to_notification(&task_id, event);
            
            let (ack_tx, ack_rx) = oneshot::channel();
            self.session_update_tx.send((notification, ack_tx))
                .map_err(|_| acp::Error::other("Failed to send notification"))?;
            
            ack_rx.await.ok();
        }
        
        Ok(acp::PromptResponse::new(StopReason::EndTurn))
    }
}

fn build_system_prompt(task_context: &TaskContext) -> String {
    format!(r#"
You are an AI task agent working on: "{}"

Task ID: {}
Description: {}
Current Status: {}
Labels: {:?}

## Your Role
Analyze the task and complete it using available tools.

## Available Tools
You have access to task management tools:
- task_comment: Add comments to the task
- update_task_labels: Add/remove labels like 'needs_clarification', 'agent_done', 'needs_review'
- update_task_status: Change task status

## Workflow
1. Analyze the task requirements
2. If unclear: ask questions via task_comment and add 'needs_clarification' label
3. Execute the work
4. When complete: 
   - Add completion comment with summary
   - Add 'agent_done' and 'needs_review' labels
   - Mark status as 'done' if appropriate

## Current Comments
{}

Proceed to work on this task.
"#,
        task_context.title,
        task_context.task_id,
        task_context.description.as_deref().unwrap_or("No description"),
        task_context.status,
        task_context.labels,
        format_comments(&task_context.comments)
    )
}
```

### Phase 3: Integrate MCP Server into AgentScheduler

#### 3.1 Update AgentScheduler
**File**: `crates/peekoo-agent-app/src/agent_scheduler.rs`

```rust
use peekoo_mcp_server::TaskMcpHandler;
use rmcp::transport::StdioServerTransport;

pub struct AgentScheduler {
    scheduler: Scheduler,
    task_service: Arc<ProductivityService>,
    notification_service: Arc<NotificationService>,
    shutdown_token: tokio_util::sync::CancellationToken,
}

async fn execute_task_acp(
    task_service: &ProductivityService,
    notification_service: &NotificationService,
    task: &TaskDto,
) -> Result<(), String> {
    let task_id = task.id.clone();
    
    // Set initial state
    task_service.update_task_status(&task_id, TaskStatus::InProgress)
        .map_err(|e| format!("Failed to update status: {}", e))?;
    task_service.add_task_label(&task_id, "agent_working")
        .map_err(|e| format!("Failed to add label: {}", e))?;
    task_service.add_task_comment(
        &task_id,
        "I'm starting work on this task. I'll analyze the requirements and let you know if I have any questions.",
        "agent"
    ).ok();
    
    // Start MCP server in background thread
    let (mcp_tx, mcp_rx) = tokio::sync::oneshot::channel();
    let task_service_clone = Arc::clone(&task_service);
    
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        
        rt.block_on(async {
            let handler = TaskMcpHandler::new(task_service_clone);
            let transport = StdioServerTransport::new();
            
            // Signal that MCP server is ready
            mcp_tx.send(()).ok();
            
            // Run server
            if let Err(e) = handler.serve(transport).await {
                tracing::error!("MCP server error: {}", e);
            }
        });
    });
    
    // Wait for MCP server to be ready
    mcp_rx.await.map_err(|e| format!("MCP server failed to start: {}", e))?;
    
    // Give MCP server a moment to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Execute agent with retry logic
    let mut attempts = 0;
    let max_attempts = 3;
    let mut last_error = None;
    
    while attempts < max_attempts {
        attempts += 1;
        tracing::info!("Agent execution attempt {} for task {}", attempts, task_id);
        
        match run_agent_acp(task_service, notification_service, task).await {
            Ok(()) => {
                tracing::info!("Task {} completed successfully", task_id);
                
                // Check if needs clarification
                let task = task_service.load_task(&task_id)
                    .map_err(|e| format!("Failed to load task: {}", e))?;
                
                if task.labels.contains("needs_clarification") {
                    notification_service.send(Notification {
                        source: "peekoo-agent".to_string(),
                        title: format!("Help Needed: {}", task.title),
                        body: "The agent has questions about this task. Please check the task comments.",
                    });
                    
                    task_service.update_agent_work_status(
                        &task_id,
                        "waiting_for_clarification",
                        None
                    ).ok();
                    
                    return Ok(());
                }
                
                // Task completed successfully
                finalize_successful_task(task_service, notification_service, &task, "").await?;
                return Ok(());
            }
            Err(e) => {
                tracing::error!("Attempt {} failed for task {}: {}", attempts, task_id, e);
                last_error = Some(e);
                
                if attempts < max_attempts {
                    tracing::info!("Retrying in 5 seconds...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
    
    // All retries exhausted
    tracing::error!("Task {} failed after {} attempts", task_id, max_attempts);
    
    finalize_failed_task(task_service, notification_service, &task, 
        &last_error.unwrap_or_else(|| "Unknown error".to_string())
    ).await?;
    
    Err(format!("Failed after {} attempts", max_attempts))
}

async fn run_agent_acp(
    task_service: &ProductivityService,
    notification_service: &NotificationService,
    task: &TaskDto,
) -> Result<(), String> {
    use agent_client_protocol::{
        Client, ClientSideConnection, ContentBlock, InitializeRequest,
        NewSessionRequest, ProtocolVersion, PromptRequest, TextContent,
    };
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
    
    // Spawn peekoo-agent-acp subprocess
    let bin_name = if cfg!(windows) { "peekoo-agent-acp.exe" } else { "peekoo-agent-acp" };
    let command_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join(bin_name)))
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from(bin_name));
    
    let mut child = Command::new(&command_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn agent: {}", e))?;
    
    let stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    
    // Build task context
    let comments = task_service.get_task_activity(&task.id, 100)
        .map_err(|e| e.to_string())?;
    
    let task_context = serde_json::json!({
        "task_id": task.id,
        "title": task.title,
        "description": task.description,
        "status": task.status,
        "priority": task.priority,
        "labels": task.labels,
        "comments": comments.iter().map(|c| {
            serde_json::json!({
                "author": c.payload.get("author").and_then(|v| v.as_str()).unwrap_or("unknown"),
                "text": c.payload.get("text").and_then(|v| v.as_str()).unwrap_or(""),
                "created_at": c.created_at
            })
        }).collect::<Vec<_>>()
    });
    
    // MCP server config (will connect to our embedded server)
    let mcp_config = serde_json::json!({
        "mcpServers": [{
            "name": "task-tools",
            "transport": "stdio"
        }]
    });
    
    // Run ACP communication in LocalSet
    let local_set = LocalSet::new();
    let result = local_set.run_until(async {
        let (conn, handle_io) = ClientSideConnection::new(
            TaskClient { task_id: task.id.clone() },
            stdin.compat_write(),
            stdout.compat(),
            |fut| { tokio::task::spawn_local(fut); },
        );
        
        tokio::task::spawn_local(async move {
            if let Err(e) = handle_io.await {
                tracing::error!("ACP I/O error: {}", e);
            }
        });
        
        // Initialize with MCP config
        let init_result = conn.initialize(
            InitializeRequest::new(ProtocolVersion::V1)
                .mcp_servers(mcp_config["mcpServers"].clone())
        ).await.map_err(|e| format!("Initialize error: {}", e))?;
        
        tracing::info!("Agent initialized: {:?}", init_result.agent_info);
        
        // Create session
        let session = conn.new_session(NewSessionRequest::new(
            std::env::current_dir().unwrap_or_default(),
        )).await.map_err(|e| format!("New session error: {}", e))?;
        
        tracing::info!("Session created: {}", session.session_id);
        
        // Send prompt with task context
        let prompt_json = serde_json::to_string(&task_context)
            .map_err(|e| e.to_string())?;
        
        let prompt_response = conn.prompt(PromptRequest::new(
            session.session_id,
            vec![ContentBlock::Text(TextContent::new(prompt_json))],
        )).await.map_err(|e| format!("Prompt error: {}", e))?;
        
        tracing::info!("Prompt completed with reason: {:?}", prompt_response.stop_reason);
        
        Ok::<_, String>(())
    }).await;
    
    // Cleanup
    drop(local_set);
    let _ = child.kill().await;
    
    result
}

async fn finalize_successful_task(
    task_service: &ProductivityService,
    notification_service: &NotificationService,
    task: &TaskDto,
    summary: &str,
) -> Result<(), String> {
    let task_id = &task.id;
    
    // Remove working label
    task_service.remove_task_label(task_id, "agent_working").ok();
    
    // Add completion labels
    task_service.add_task_label(task_id, "agent_done").ok();
    task_service.add_task_label(task_id, "needs_review").ok();
    
    // Add completion comment
    task_service.add_task_comment(
        task_id,
        &format!("Task completed.\n\n**Summary:** {}", summary),
        "agent"
    ).ok();
    
    // Update work status
    task_service.update_agent_work_status(task_id, "completed", None).ok();
    
    // Notify user
    notification_service.send(Notification {
        source: "peekoo-agent".to_string(),
        title: format!("Task Complete: {}", task.title),
        body: "The agent has completed the task and added a 'needs_review' label.",
    });
    
    Ok(())
}

async fn finalize_failed_task(
    task_service: &ProductivityService,
    notification_service: &NotificationService,
    task: &TaskDto,
    error: &str,
) -> Result<(), String> {
    let task_id = &task.id;
    
    // Remove working label
    task_service.remove_task_label(task_id, "agent_working").ok();
    
    // Add failure label
    task_service.add_task_label(task_id, "agent_failed").ok();
    
    // Add failure comment
    task_service.add_task_comment(
        task_id,
        &format!("Task failed after multiple attempts.\n\n**Error:** {}", error),
        "agent"
    ).ok();
    
    // Update work status
    task_service.update_agent_work_status(task_id, "failed", None).ok();
    
    // Notify user
    notification_service.send(Notification {
        source: "peekoo-agent".to_string(),
        title: format!("Task Failed: {}", task.title),
        body: "The agent failed to complete the task after multiple attempts.",
    });
    
    Ok(())
}
```

### Phase 4: Add TaskService Methods

#### 4.1 Update TaskService trait
**File**: `crates/peekoo-productivity-domain/src/task.rs`

```rust
pub trait TaskService: Send + Sync {
    // ... existing methods ...
    
    fn add_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String>;
    fn remove_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String>;
    fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<TaskDto, String>;
    fn load_task(&self, task_id: &str) -> Result<TaskDto, String>;
}
```

#### 4.2 Implement in ProductivityService
**File**: `crates/peekoo-agent-app/src/productivity.rs`

```rust
impl ProductivityService {
    pub fn add_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String> {
        let mut task = self.load_task(task_id)?;
        
        if !task.labels.contains(&label.to_string()) {
            task.labels.push(label.to_string());
            self.save_task(&task)?;
            
            // Emit event
            self.write_event(&task.id, TaskEventType::Labeled, json!({"label": label}))?;
        }
        
        Ok(task.into())
    }
    
    pub fn remove_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String> {
        let mut task = self.load_task(task_id)?;
        
        task.labels.retain(|l| l != label);
        self.save_task(&task)?;
        
        // Emit event
        self.write_event(&task.id, TaskEventType::Unlabeled, json!({"label": label}))?;
        
        Ok(task.into())
    }
    
    pub fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<TaskDto, String> {
        let mut task = self.load_task(task_id)?;
        let old_status = task.status.clone();
        
        task.status = status.clone();
        self.save_task(&task)?;
        
        // Emit event
        self.write_event(
            &task.id,
            TaskEventType::StatusChanged,
            json!({
                "old_status": old_status,
                "new_status": status
            })
        )?;
        
        Ok(task.into())
    }
    
    pub fn load_task(&self, task_id: &str) -> Result<Task, String> {
        self.conn.lock().map_err(|e| e.to_string())?
            .query_row(
                "SELECT data_json FROM tasks WHERE id = ?1",
                [task_id],
                |row| {
                    let data: String = row.get(0)?;
                    Ok(serde_json::from_str::<Task>(&data).map_err(|e| e.to_string())?)
                }
            )
            .map_err(|e| e.to_string())
    }
    
    fn save_task(&self, task: &Task) -> Result<(), String> {
        self.conn.lock().map_err(|e| e.to_string())?
            .execute(
                "UPDATE tasks SET data_json = ?1, updated_at = ?2 WHERE id = ?3",
                [
                    serde_json::to_string(task).map_err(|e| e.to_string())?,
                    Utc::now().to_rfc3339(),
                    task.id.clone(),
                ]
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
```

### Phase 5: Frontend Updates

#### 5.1 Add predefined labels
**File**: `apps/desktop-ui/src/types/task.ts`

```typescript
export const PREDEFINED_LABELS = [
  { name: "bug", color: "#E5484D" },
  { name: "feature", color: "#30A46C" },
  { name: "urgent", color: "#E9762B" },
  { name: "design", color: "#7B61FF" },
  { name: "docs", color: "#7B9AC7" },
  { name: "refactor", color: "#F5C842" },
  // Agent-specific labels
  { name: "agent_working", color: "#3B82F6" },      // Blue
  { name: "needs_clarification", color: "#F59E0B" }, // Amber
  { name: "agent_done", color: "#10B981" },        // Green
  { name: "needs_review", color: "#8B5CF6" },      // Purple
  { name: "agent_failed", color: "#EF4444" },      // Red
] as const;
```

### Phase 6: Update Workspace Configuration

**File**: `Cargo.toml` (workspace root)

Add to workspace members:
```toml
members = [
    # ... existing crates ...
    "crates/peekoo-mcp-server",
]
```

## Files Summary

### New Files
1. `crates/peekoo-mcp-server/Cargo.toml`
2. `crates/peekoo-mcp-server/src/lib.rs`
3. `crates/peekoo-mcp-server/src/main.rs` (optional, for testing)
4. `crates/peekoo-agent-acp/src/mcp_tools.rs`

### Modified Files
1. `crates/peekoo-mcp-server/src/lib.rs` - MCP server implementation
2. `crates/peekoo-agent-acp/Cargo.toml` - Add peekoo-agent, rmcp deps
3. `crates/peekoo-agent-acp/src/agent.rs` - Real LLM + MCP integration
4. `crates/peekoo-agent-acp/src/mcp_tools.rs` - MCP→pi Tool adapter
5. `crates/peekoo-agent-app/src/agent_scheduler.rs` - MCP server startup
6. `crates/peekoo-productivity-domain/src/task.rs` - New trait methods
7. `crates/peekoo-agent-app/src/productivity.rs` - Trait implementations
8. `apps/desktop-ui/src/types/task.ts` - New labels
9. `Cargo.toml` - Add workspace member

## Testing Checklist

- [ ] MCP server starts and exposes tools
- [ ] Agent connects to MCP and can list tools
- [ ] Agent calls task_comment successfully
- [ ] Agent calls update_task_labels successfully
- [ ] Agent calls update_task_status successfully
- [ ] Full task execution flow works end-to-end
- [ ] Retry logic works (3 attempts)
- [ ] Failure after retries adds agent_failed label
- [ ] Notifications sent on questions and completion
- [ ] Frontend shows new labels correctly

## Estimated Effort

- Phase 1 (MCP Server): 3 hours
- Phase 2 (Real Agent): 4 hours
- Phase 3 (Integration): 3 hours
- Phase 4 (TaskService): 1 hour
- Phase 5 (Frontend): 30 min
- Phase 6 (Testing): 2 hours
- **Total: ~14 hours**
