//! MCP Server handler implementation for task tools

use peekoo_productivity_domain::task::{TaskService, TaskStatus};
use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

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
        Parameters(params): Parameters<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_request("Missing task_id", None))?;
        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_request("Missing text", None))?;

        match self.task_service.add_task_comment(task_id, text, "agent") {
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
        Parameters(params): Parameters<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_request("Missing task_id", None))?;

        let mut errors = Vec::new();

        if let Some(labels) = params.get("add_labels").and_then(|v| v.as_array()) {
            for label in labels {
                if let Some(label_str) = label.as_str() {
                    if let Err(e) = self.task_service.add_task_label(task_id, label_str) {
                        errors.push(format!("Failed to add label '{}': {}", label_str, e));
                    }
                }
            }
        }

        if let Some(labels) = params.get("remove_labels").and_then(|v| v.as_array()) {
            for label in labels {
                if let Some(label_str) = label.as_str() {
                    if let Err(e) = self.task_service.remove_task_label(task_id, label_str) {
                        errors.push(format!("Failed to remove label '{}': {}", label_str, e));
                    }
                }
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
        Parameters(params): Parameters<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_request("Missing task_id", None))?;
        let status = params
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_request("Missing status", None))?;

        let task_status = match status {
            "pending" => TaskStatus::Todo,
            "in_progress" => TaskStatus::InProgress,
            "done" => TaskStatus::Done,
            "cancelled" => TaskStatus::Cancelled,
            _ => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid status '{}'. Must be one of: pending, in_progress, done, cancelled",
                    status
                ))]));
            }
        };

        match self.task_service.update_task_status(task_id, task_status) {
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
