use std::sync::Arc;

use async_trait::async_trait;
use peekoo_productivity_domain::task::TaskService;
use pi::error::Result;
use pi::model::{ContentBlock, TextContent};
use pi::tools::{Tool, ToolOutput, ToolUpdate};

fn ok_json(value: serde_json::Value) -> Result<ToolOutput> {
    Ok(ToolOutput {
        content: vec![ContentBlock::Text(TextContent::new(
            serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string()),
        ))],
        details: None,
        is_error: false,
    })
}

fn err_json(msg: impl Into<String>) -> Result<ToolOutput> {
    Ok(ToolOutput {
        content: vec![ContentBlock::Text(TextContent::new(msg.into()))],
        details: None,
        is_error: true,
    })
}

// ── task_create ──────────────────────────────────────────────────────

pub struct CreateTaskTool {
    service: Arc<dyn TaskService>,
}

impl CreateTaskTool {
    pub fn new(service: Arc<dyn TaskService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for CreateTaskTool {
    fn name(&self) -> &str {
        "task_create"
    }
    fn label(&self) -> &str {
        "task_create"
    }
    fn description(&self) -> &str {
        "Create a new task with title, optional priority (low/medium/high), optional assignee (user/agent), and optional labels array."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Task title" },
                "priority": { "type": "string", "enum": ["low", "medium", "high"], "description": "Priority level, defaults to medium" },
                "assignee": { "type": "string", "enum": ["user", "agent"], "description": "Who the task is assigned to, defaults to user" },
                "labels": { "type": "array", "items": { "type": "string" }, "description": "Optional labels" }
            },
            "required": ["title"]
        })
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let title = input["title"].as_str().unwrap_or("");
        let priority = input["priority"].as_str().unwrap_or("medium");
        let assignee = input["assignee"].as_str().unwrap_or("user");
        let labels: Vec<String> = input["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        match self.service.create_task(title, priority, assignee, &labels) {
            Ok(dto) => ok_json(serde_json::to_value(&dto).unwrap_or_default()),
            Err(e) => err_json(format!("Create task error: {e}")),
        }
    }
}

// ── task_list ────────────────────────────────────────────────────────

pub struct ListTasksTool {
    service: Arc<dyn TaskService>,
}

impl ListTasksTool {
    pub fn new(service: Arc<dyn TaskService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for ListTasksTool {
    fn name(&self) -> &str {
        "task_list"
    }
    fn label(&self) -> &str {
        "task_list"
    }
    fn description(&self) -> &str {
        "List all tasks. Optionally filter by status (todo/in_progress/done)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "status_filter": { "type": "string", "enum": ["todo", "in_progress", "done"], "description": "Optional status filter" }
            }
        })
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        match self.service.list_tasks() {
            Ok(tasks) => {
                let filtered: Vec<_> = match input["status_filter"].as_str() {
                    Some(status) => tasks.into_iter().filter(|t| t.status == status).collect(),
                    None => tasks,
                };
                ok_json(serde_json::to_value(&filtered).unwrap_or_default())
            }
            Err(e) => err_json(format!("List tasks error: {e}")),
        }
    }
}

// ── task_update ──────────────────────────────────────────────────────

pub struct UpdateTaskTool {
    service: Arc<dyn TaskService>,
}

impl UpdateTaskTool {
    pub fn new(service: Arc<dyn TaskService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for UpdateTaskTool {
    fn name(&self) -> &str {
        "task_update"
    }
    fn label(&self) -> &str {
        "task_update"
    }
    fn description(&self) -> &str {
        "Update a task's title, priority, status, assignee, or labels. Provide the task id and any fields to change."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Task ID" },
                "title": { "type": "string", "description": "New title" },
                "priority": { "type": "string", "enum": ["low", "medium", "high"] },
                "status": { "type": "string", "enum": ["todo", "in_progress", "done"] },
                "assignee": { "type": "string", "enum": ["user", "agent"] },
                "labels": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let id = input["id"].as_str().unwrap_or("");
        let title = input["title"].as_str();
        let priority = input["priority"].as_str();
        let status = input["status"].as_str();
        let assignee = input["assignee"].as_str();
        let labels: Option<Vec<String>> = input["labels"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });
        let labels_ref = labels.as_deref();

        match self
            .service
            .update_task(id, title, priority, status, assignee, labels_ref)
        {
            Ok(dto) => ok_json(serde_json::to_value(&dto).unwrap_or_default()),
            Err(e) => err_json(format!("Update task error: {e}")),
        }
    }
}

// ── task_delete ──────────────────────────────────────────────────────

pub struct DeleteTaskTool {
    service: Arc<dyn TaskService>,
}

impl DeleteTaskTool {
    pub fn new(service: Arc<dyn TaskService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for DeleteTaskTool {
    fn name(&self) -> &str {
        "task_delete"
    }
    fn label(&self) -> &str {
        "task_delete"
    }
    fn description(&self) -> &str {
        "Delete a task by its ID."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Task ID to delete" }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let id = input["id"].as_str().unwrap_or("");
        match self.service.delete_task(id) {
            Ok(()) => ok_json(serde_json::json!({"ok": true})),
            Err(e) => err_json(format!("Delete task error: {e}")),
        }
    }
}

// ── task_toggle ──────────────────────────────────────────────────────

pub struct ToggleTaskTool {
    service: Arc<dyn TaskService>,
}

impl ToggleTaskTool {
    pub fn new(service: Arc<dyn TaskService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for ToggleTaskTool {
    fn name(&self) -> &str {
        "task_toggle"
    }
    fn label(&self) -> &str {
        "task_toggle"
    }
    fn description(&self) -> &str {
        "Toggle a task's completion status (todo <-> done)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Task ID to toggle" }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let id = input["id"].as_str().unwrap_or("");
        match self.service.toggle_task(id) {
            Ok(dto) => ok_json(serde_json::to_value(&dto).unwrap_or_default()),
            Err(e) => err_json(format!("Toggle task error: {e}")),
        }
    }
}

// ── task_assign ──────────────────────────────────────────────────────

pub struct AssignTaskTool {
    service: Arc<dyn TaskService>,
}

impl AssignTaskTool {
    pub fn new(service: Arc<dyn TaskService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for AssignTaskTool {
    fn name(&self) -> &str {
        "task_assign"
    }
    fn label(&self) -> &str {
        "task_assign"
    }
    fn description(&self) -> &str {
        "Assign a task to a user or agent."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Task ID" },
                "assignee": { "type": "string", "enum": ["user", "agent"], "description": "Who to assign to" }
            },
            "required": ["id", "assignee"]
        })
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let id = input["id"].as_str().unwrap_or("");
        let assignee = input["assignee"].as_str().unwrap_or("user");
        match self
            .service
            .update_task(id, None, None, None, Some(assignee), None)
        {
            Ok(dto) => ok_json(serde_json::to_value(&dto).unwrap_or_default()),
            Err(e) => err_json(format!("Assign task error: {e}")),
        }
    }
}

// ── Factory ──────────────────────────────────────────────────────────

pub fn create_task_tools(service: Arc<dyn TaskService>) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(CreateTaskTool::new(Arc::clone(&service))),
        Box::new(ListTasksTool::new(Arc::clone(&service))),
        Box::new(UpdateTaskTool::new(Arc::clone(&service))),
        Box::new(DeleteTaskTool::new(Arc::clone(&service))),
        Box::new(ToggleTaskTool::new(Arc::clone(&service))),
        Box::new(AssignTaskTool::new(service)),
    ]
}
