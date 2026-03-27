use std::sync::Arc;

use async_trait::async_trait;
use peekoo_task_app::TaskService;
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
        "Create a new task. Supports title, priority, assignee, labels, description, scheduling (start/end times), estimated duration, and recurrence rules (RRULE format like 'FREQ=DAILY' or 'FREQ=WEEKLY;BYDAY=MO,WE,FR')."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Task title" },
                "priority": { "type": "string", "enum": ["low", "medium", "high"], "description": "Priority level, defaults to medium" },
                "assignee": { "type": "string", "enum": ["user", "agent"], "description": "Who the task is assigned to, defaults to user" },
                "labels": { "type": "array", "items": { "type": "string" }, "description": "Optional labels" },
                "description": { "type": "string", "description": "Detailed description or notes for the task (supports markdown)" },
                "scheduled_start_at": { "type": "string", "description": "ISO 8601 start time (e.g., '2026-03-20T14:00:00Z')" },
                "scheduled_end_at": { "type": "string", "description": "ISO 8601 end time" },
                "estimated_duration_min": { "type": "integer", "description": "Estimated duration in minutes" },
                "recurrence_rule": { "type": "string", "description": "RRULE string for recurring tasks (e.g., 'FREQ=DAILY', 'FREQ=WEEKLY;BYDAY=MO,WE,FR')" }
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
        let description = input["description"].as_str();
        let scheduled_start_at = input["scheduled_start_at"].as_str();
        let scheduled_end_at = input["scheduled_end_at"].as_str();
        let estimated_duration_min = input["estimated_duration_min"].as_u64().map(|v| v as u32);
        let recurrence_rule = input["recurrence_rule"].as_str();
        let recurrence_time_of_day = input["recurrence_time_of_day"].as_str();

        match self.service.create_task(
            title,
            priority,
            assignee,
            &labels,
            description,
            scheduled_start_at,
            scheduled_end_at,
            estimated_duration_min,
            recurrence_rule,
            recurrence_time_of_day,
        ) {
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
        "Update a task's title, priority, status, assignee, labels, description, scheduling, or recurrence. Provide the task id and any fields to change."
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
                "labels": { "type": "array", "items": { "type": "string" } },
                "description": { "type": "string", "description": "New description/notes (supports markdown)" },
                "scheduled_start_at": { "type": "string", "description": "ISO 8601 start time" },
                "scheduled_end_at": { "type": "string", "description": "ISO 8601 end time" },
                "estimated_duration_min": { "type": "integer", "description": "Estimated duration in minutes" },
                "recurrence_rule": { "type": "string", "description": "RRULE string (e.g., 'FREQ=DAILY')" }
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
        let description = input["description"].as_str();
        let scheduled_start_at = input["scheduled_start_at"].as_str();
        let scheduled_end_at = input["scheduled_end_at"].as_str();
        let estimated_duration_min: Option<Option<u32>> =
            if input.get("estimated_duration_min").is_some() {
                Some(input["estimated_duration_min"].as_u64().map(|v| v as u32))
            } else {
                None
            };
        let recurrence_rule: Option<Option<&str>> = if input.get("recurrence_rule").is_some() {
            Some(input["recurrence_rule"].as_str())
        } else {
            None
        };
        let recurrence_time_of_day: Option<Option<&str>> =
            if input.get("recurrence_time_of_day").is_some() {
                Some(input["recurrence_time_of_day"].as_str())
            } else {
                None
            };

        match self.service.update_task(
            id,
            title,
            priority,
            status,
            assignee,
            labels_ref,
            description,
            scheduled_start_at,
            scheduled_end_at,
            estimated_duration_min,
            recurrence_rule,
            recurrence_time_of_day,
        ) {
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
        match self.service.update_task(
            id,
            None,
            None,
            None,
            Some(assignee),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ) {
            Ok(dto) => ok_json(serde_json::to_value(&dto).unwrap_or_default()),
            Err(e) => err_json(format!("Assign task error: {e}")),
        }
    }
}

// ── task_comment ──────────────────────────────────────────────────────

pub struct CommentTaskTool {
    service: Arc<dyn TaskService>,
}

impl CommentTaskTool {
    pub fn new(service: Arc<dyn TaskService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for CommentTaskTool {
    fn name(&self) -> &str {
        "task_comment"
    }
    fn label(&self) -> &str {
        "task_comment"
    }
    fn description(&self) -> &str {
        "Add a comment to a task. Comments support markdown formatting. Use this to provide updates, ask questions, or leave notes on tasks."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID to comment on" },
                "text": { "type": "string", "description": "Comment text (supports markdown)" }
            },
            "required": ["task_id", "text"]
        })
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let task_id = input["task_id"].as_str().unwrap_or("");
        let text = input["text"].as_str().unwrap_or("");
        match self.service.add_task_comment(task_id, text, "peekoo-agent") {
            Ok(event) => ok_json(serde_json::to_value(&event).unwrap_or_default()),
            Err(e) => err_json(format!("Add comment error: {e}")),
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
        Box::new(AssignTaskTool::new(Arc::clone(&service))),
        Box::new(CommentTaskTool::new(service)),
    ]
}
