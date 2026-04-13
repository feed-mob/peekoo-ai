//! MCP Server handler implementation for Peekoo native tools (task, pomodoro, settings)

use peekoo_app_settings::{
    AppSettingsService, GenerateSpriteManifestInput, SaveCustomSpriteInput,
    ValidateSpriteManifestInput,
};
use peekoo_pomodoro_app::{PomodoroAppService, PomodoroSettingsInput};
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

// ── Pomodoro Parameter types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PomodoroStartParams {
    mode: String,
    minutes: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PomodoroSwitchModeParams {
    mode: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PomodoroSaveMemoParams {
    id: Option<String>,
    memo: String,
    task_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PomodoroHistoryParams {
    #[serde(default = "default_history_limit")]
    limit: usize,
}

fn default_history_limit() -> usize {
    10
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PomodoroHistoryByDateRangeParams {
    start_date: String,
    end_date: String,
    #[serde(default = "default_history_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PomodoroSetSettingsParams {
    work_minutes: u32,
    break_minutes: u32,
    long_break_minutes: u32,
    long_break_interval: u32,
    enable_memo: bool,
    auto_advance: bool,
}

// ── Settings Parameter types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetActiveSpriteParams {
    sprite_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetThemeParams {
    mode: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GenerateSpriteManifestParams {
    image_path: String,
    name: String,
    description: Option<String>,
    columns: u32,
    rows: u32,
    scale: Option<f32>,
    frame_rate: Option<u32>,
    use_chroma_key: bool,
    pixel_art: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ValidateSpriteManifestParams {
    image_path: String,
    manifest: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SaveCustomSpriteParams {
    image_path: String,
    manifest: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DeleteCustomSpriteParams {
    sprite_id: String,
}

// ── Handler ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TaskMcpHandler {
    task_service: Arc<dyn TaskService>,
    pomodoro_service: Arc<PomodoroAppService>,
    settings_service: Arc<AppSettingsService>,
    tool_router: ToolRouter<TaskMcpHandler>,
}

impl TaskMcpHandler {
    pub fn new(
        task_service: Arc<dyn TaskService>,
        pomodoro_service: Arc<PomodoroAppService>,
        settings_service: Arc<AppSettingsService>,
    ) -> Self {
        Self {
            task_service,
            pomodoro_service,
            settings_service,
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

    #[tool(
        name = "task_assign",
        description = "Assign a task to a user or agent."
    )]
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

    // ── Pomodoro ─────────────────────────────────────────────────────────────

    #[tool(
        name = "pomodoro_status",
        description = "Get the current pomodoro timer status including mode, time remaining, and daily stats."
    )]
    async fn pomodoro_status(&self) -> Result<CallToolResult, McpError> {
        match self.pomodoro_service.get_status() {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "pomodoro_start",
        description = "Start a new pomodoro session. Mode can be 'focus' or 'break'. Minutes specifies duration."
    )]
    async fn pomodoro_start(
        &self,
        Parameters(params): Parameters<PomodoroStartParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.pomodoro_service.start(&params.mode, params.minutes) {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "pomodoro_pause",
        description = "Pause the currently active pomodoro timer."
    )]
    async fn pomodoro_pause(&self) -> Result<CallToolResult, McpError> {
        match self.pomodoro_service.pause() {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "pomodoro_resume",
        description = "Resume a paused pomodoro timer."
    )]
    async fn pomodoro_resume(&self) -> Result<CallToolResult, McpError> {
        match self.pomodoro_service.resume() {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "pomodoro_finish",
        description = "Finish or cancel the current pomodoro session."
    )]
    async fn pomodoro_finish(&self) -> Result<CallToolResult, McpError> {
        match self.pomodoro_service.finish() {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "pomodoro_switch_mode",
        description = "Switch between focus and break modes. Mode can be 'focus' or 'break'."
    )]
    async fn pomodoro_switch_mode(
        &self,
        Parameters(params): Parameters<PomodoroSwitchModeParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.pomodoro_service.switch_mode(&params.mode) {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "pomodoro_save_memo",
        description = "Save a memo for a pomodoro session. id is optional (uses current session if not provided)."
    )]
    async fn pomodoro_save_memo(
        &self,
        Parameters(params): Parameters<PomodoroSaveMemoParams>,
    ) -> Result<CallToolResult, McpError> {
        let task_title = params
            .task_id
            .as_deref()
            .map(|task_id| self.task_service.load_task(task_id).map(|task| task.title))
            .transpose();

        let task_title = match task_title {
            Ok(task_title) => task_title,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(e)])),
        };

        match self.pomodoro_service.save_pomodoro_memo(
            params.id,
            params.memo,
            params.task_id,
            task_title,
        ) {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "pomodoro_history",
        description = "Get pomodoro session history. Defaults to last 10 sessions."
    )]
    async fn pomodoro_history(
        &self,
        Parameters(params): Parameters<PomodoroHistoryParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.pomodoro_service.history(params.limit) {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "pomodoro_history_by_date_range",
        description = "Get pomodoro sessions within a date range (YYYY-MM-DD format). Defaults to last 10."
    )]
    async fn pomodoro_history_by_date_range(
        &self,
        Parameters(params): Parameters<PomodoroHistoryByDateRangeParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.pomodoro_service.history_by_date_range(
            &params.start_date,
            &params.end_date,
            params.limit,
        ) {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "pomodoro_set_settings",
        description = "Configure pomodoro settings: work duration, break duration, long break settings, and behavior options."
    )]
    async fn pomodoro_set_settings(
        &self,
        Parameters(params): Parameters<PomodoroSetSettingsParams>,
    ) -> Result<CallToolResult, McpError> {
        let input = PomodoroSettingsInput {
            work_minutes: params.work_minutes,
            break_minutes: params.break_minutes,
            long_break_minutes: params.long_break_minutes,
            long_break_interval: params.long_break_interval,
            enable_memo: params.enable_memo,
            auto_advance: params.auto_advance,
        };
        match self.pomodoro_service.set_settings(input) {
            Ok(dto) => json_success(dto),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    // ── Settings ───────────────────────────────────────────────────────────

    #[tool(
        name = "settings_get_active_sprite",
        description = "Get the currently active character (sprite) ID."
    )]
    async fn settings_get_active_sprite(&self) -> Result<CallToolResult, McpError> {
        match self.settings_service.get_active_sprite_id() {
            Ok(id) => json_success(serde_json::json!({ "sprite_id": id })),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "settings_set_active_sprite",
        description = "Set the active character (sprite). Use settings_list_sprites to see available options."
    )]
    async fn settings_set_active_sprite(
        &self,
        Parameters(params): Parameters<SetActiveSpriteParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .settings_service
            .set_active_sprite_id(&params.sprite_id)
        {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                "Sprite updated",
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "settings_list_sprites",
        description = "List all available characters (sprites) with their IDs and descriptions."
    )]
    async fn settings_list_sprites(&self) -> Result<CallToolResult, McpError> {
        let sprites = self.settings_service.list_sprites();
        json_success(sprites)
    }

    #[tool(
        name = "settings_get_sprite_prompt",
        description = "Get the recommended prompt template for generating a Peekoo sprite sheet."
    )]
    async fn settings_get_sprite_prompt(&self) -> Result<CallToolResult, McpError> {
        json_success(serde_json::json!({ "prompt": self.settings_service.get_sprite_prompt() }))
    }

    #[tool(
        name = "settings_get_sprite_manifest_template",
        description = "Get an example sprite manifest template for custom sprites."
    )]
    async fn settings_get_sprite_manifest_template(&self) -> Result<CallToolResult, McpError> {
        json_success(self.settings_service.get_sprite_manifest_template())
    }

    #[tool(
        name = "settings_generate_sprite_manifest_draft",
        description = "Generate a starter manifest draft and image validation report for an uploaded sprite image path."
    )]
    async fn settings_generate_sprite_manifest_draft(
        &self,
        Parameters(params): Parameters<GenerateSpriteManifestParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .settings_service
            .generate_sprite_manifest_draft(GenerateSpriteManifestInput {
                image_path: params.image_path,
                name: params.name,
                description: params.description,
                columns: params.columns,
                rows: params.rows,
                scale: params.scale,
                frame_rate: params.frame_rate,
                use_chroma_key: params.use_chroma_key,
                pixel_art: params.pixel_art,
            }) {
            Ok(result) => json_success(result),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "settings_validate_sprite_manifest",
        description = "Validate a custom sprite manifest JSON object against an uploaded sprite image path."
    )]
    async fn settings_validate_sprite_manifest(
        &self,
        Parameters(params): Parameters<ValidateSpriteManifestParams>,
    ) -> Result<CallToolResult, McpError> {
        let manifest = match serde_json::from_value(params.manifest) {
            Ok(manifest) => manifest,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid manifest JSON: {e}"
                ))]));
            }
        };

        match self
            .settings_service
            .validate_manifest(&ValidateSpriteManifestInput {
                image_path: params.image_path,
                manifest,
            }) {
            Ok(result) => json_success(result),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "settings_save_custom_sprite",
        description = "Save a validated custom sprite from a local image path and manifest JSON object."
    )]
    async fn settings_save_custom_sprite(
        &self,
        Parameters(params): Parameters<SaveCustomSpriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let manifest = match serde_json::from_value(params.manifest) {
            Ok(manifest) => manifest,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid manifest JSON: {e}"
                ))]));
            }
        };

        match self
            .settings_service
            .save_custom_sprite(SaveCustomSpriteInput {
                image_path: params.image_path,
                manifest,
            }) {
            Ok(result) => json_success(result),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "settings_delete_custom_sprite",
        description = "Delete a saved custom sprite by ID. Built-in sprites cannot be deleted."
    )]
    async fn settings_delete_custom_sprite(
        &self,
        Parameters(params): Parameters<DeleteCustomSpriteParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .settings_service
            .delete_custom_sprite(&params.sprite_id)
        {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                "Custom sprite deleted",
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "settings_get_theme",
        description = "Get the current theme mode: 'light', 'dark', or 'system'."
    )]
    async fn settings_get_theme(&self) -> Result<CallToolResult, McpError> {
        match self.settings_service.get_theme_mode() {
            Ok(mode) => json_success(serde_json::json!({ "mode": mode })),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(
        name = "settings_set_theme",
        description = "Set the theme mode. Valid values: 'light', 'dark', 'system'."
    )]
    async fn settings_set_theme(
        &self,
        Parameters(params): Parameters<SetThemeParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.settings_service.set_theme_mode(&params.mode) {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                "Theme updated",
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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn json_success(value: impl serde::Serialize) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}
