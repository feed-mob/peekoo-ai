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

#[derive(Clone)]
pub struct TaskMcpHandler {
    task_service: std::sync::Arc<dyn TaskService>,
    tool_router: ToolRouter<TaskMcpHandler>,
}

impl TaskMcpHandler {
    pub fn new(task_service: std::sync::Arc<dyn TaskService>) -> Self {
        Self {
            task_service,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl TaskMcpHandler {
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
            Ok(CallToolResult::success(vec![Content::text(
                "Labels updated",
            )]))
        } else {
            Ok(CallToolResult::error(vec![Content::text(
                errors.join("; "),
            )]))
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
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                "Status updated",
            )])),
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
