//! MCP Server handler implementation for task tools

use peekoo_task_app::TaskService;
use peekoo_task_domain::TaskStatus;
use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars::JsonSchema,
    tool, tool_handler, tool_router,
};
use serde::Deserialize;
use std::sync::Arc;

// ── Parameter types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TaskCreateParams {
    title: String,
    #[serde(default = "default_priority")]
    priority: String,
    #[serde(default = "default_assignee")]
    assignee: String,
    #[serde(default)]
    labels: Vec<String>,
    description: Option<String>,
    scheduled_start_at: Option<String>,
    scheduled_end_at: Option<String>,
    estimated_duration_min: Option<u32>,
    recurrence_rule: Option<String>,
}

fn default_priority() -> String {
    "medium".to_string()
}
fn default_assignee() -> String {
    "user".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TaskListParams {
    status_filter: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TaskUpdateParams {
    id: String,
    title: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
    description: Option<String>,
    scheduled_start_at: Option<String>,
    scheduled_end_at: Option<String>,
    estimated_duration_min: Option<u32>,
    recurrence_rule: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TaskIdParams {
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TaskAssignParams {
    id: String,
    assignee: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TaskCommentParams {
    task_id: String,
    text: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UpdateTaskLabelsParams {
    task_id: String,
    #[serde(default)]
    add_labels: Vec<String>,
    #[serde(default)]
    remove_labels: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UpdateTaskStatusParams {
    task_id: String,
    status: String,
}

// ── Handler ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TaskMcpHandler {
    task_service: Arc<dyn TaskService>,
    tool_router: ToolRouter<TaskMcpHandler>,
}

impl TaskMcpHandler {
    pub fn new(task_service: Arc<dyn TaskService>) -> Self {
        Self {
            task_service,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl TaskMcpHandler {
    // ── Task CRUD ─────────────────────────────────────────────────────────────

    #[tool(
        name = "task_create",
        description = "Create a new task. Supports title, priority, assignee, labels, description, scheduling, and recurrence rules."
    )]
    async fn task_create(
        &self,
        Parameters(params): Parameters<TaskCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.task_service.create_task(
            &params.title,
            &params.priority,
            &params.assignee,
            &params.labels,
            params.description.as_deref(),
            params.scheduled_start_at.as_deref(),
            params.scheduled_end_at.as_deref(),
            params.estimated_duration_min,
            params.recurrence_rule.as_deref(),
            None,
        ) {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "task_list",
        description = "List all tasks. Optionally filter by status (todo/in_progress/done)."
    )]
    async fn task_list(
        &self,
        Parameters(params): Parameters<TaskListParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.task_service.list_tasks() {
            Ok(tasks) => {
                let filtered: Vec<_> = match params.status_filter.as_deref() {
                    Some(status) => tasks.into_iter().filter(|t| t.status == status).collect(),
                    None => tasks,
                };
                json_success(filtered)
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "task_update",
        description = "Update a task's title, priority, status, assignee, labels, description, scheduling, or recurrence."
    )]
    async fn task_update(
        &self,
        Parameters(params): Parameters<TaskUpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        let labels_ref = params.labels.as_deref();
        let estimated = params.estimated_duration_min.map(Some);
        let recurrence = params.recurrence_rule.as_deref().map(Some);

        match self.task_service.update_task(
            &params.id,
            params.title.as_deref(),
            params.priority.as_deref(),
            params.status.as_deref(),
            params.assignee.as_deref(),
            labels_ref,
            params.description.as_deref(),
            params.scheduled_start_at.as_deref(),
            params.scheduled_end_at.as_deref(),
            estimated,
            recurrence,
            None,
        ) {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(name = "task_delete", description = "Delete a task by its ID.")]
    async fn task_delete(
        &self,
        Parameters(params): Parameters<TaskIdParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.task_service.delete_task(&params.id) {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text("Task deleted")])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "task_toggle",
        description = "Toggle a task's completion status (todo <-> done)."
    )]
    async fn task_toggle(
        &self,
        Parameters(params): Parameters<TaskIdParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.task_service.toggle_task(&params.id) {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(name = "task_assign", description = "Assign a task to a user or agent.")]
    async fn task_assign(
        &self,
        Parameters(params): Parameters<TaskAssignParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.task_service.update_task(
            &params.id,
            None,
            None,
            None,
            Some(&params.assignee),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ) {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    // ── Task activity ─────────────────────────────────────────────────────────

    #[tool(
        name = "task_comment",
        description = "Add a comment to a task. Use this to ask questions or provide updates."
    )]
    async fn task_comment(
        &self,
        Parameters(params): Parameters<TaskCommentParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .task_service
            .add_task_comment(&params.task_id, &params.text, "peekoo-agent")
        {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                "Comment added successfully",
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "update_task_labels",
        description = "Add or remove labels from a task. Use to mark state like 'needs_clarification', 'agent_done', 'needs_review'."
    )]
    async fn update_task_labels(
        &self,
        Parameters(params): Parameters<UpdateTaskLabelsParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut errors = Vec::new();

        for label in &params.add_labels {
            if let Err(e) = self.task_service.add_task_label(&params.task_id, label) {
                errors.push(format!("Failed to add label '{}': {}", label, e));
            }
        }

        for label in &params.remove_labels {
            if let Err(e) = self.task_service.remove_task_label(&params.task_id, label) {
                errors.push(format!("Failed to remove label '{}': {}", label, e));
            }
        }

        if errors.is_empty() {
            Ok(CallToolResult::success(vec![Content::text("Labels updated")]))
        } else {
            Ok(CallToolResult::error(vec![Content::text(errors.join("; "))]))
        }
    }

    #[tool(
        name = "update_task_status",
        description = "Update task status. Use to mark as 'in_progress', 'done', 'cancelled'."
    )]
    async fn update_task_status(
        &self,
        Parameters(params): Parameters<UpdateTaskStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let task_status = match params.status.as_str() {
            "pending" => TaskStatus::Todo,
            "in_progress" => TaskStatus::InProgress,
            "done" => TaskStatus::Done,
            "cancelled" => TaskStatus::Cancelled,
            _ => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid status '{}'. Must be one of: pending, in_progress, done, cancelled",
                    params.status
                ))]));
            }
        };

        match self
            .task_service
            .update_task_status(&params.task_id, task_status)
        {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text("Status updated")])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }
}

#[tool_handler]
impl rmcp::ServerHandler for TaskMcpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn json_success(value: impl serde::Serialize) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}
