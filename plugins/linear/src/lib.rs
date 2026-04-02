#![cfg_attr(not(test), no_main)]

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use peekoo_plugin_sdk::prelude::*;
use serde_json::json;

const API_KEY_KEY: &str = "linear-api-key";
const STATE_KEY: &str = "linear-state";
const SYNC_SCHEDULE_KEY: &str = "linear-sync";

const LINEAR_GRAPHQL_URL: &str = "https://api.linear.app/graphql";
const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 300;
const MAX_REMOTE_PAGES_PER_RUN: usize = 4;
const MAX_REMOTE_ISSUES_PER_RUN: usize = 200;
const MAX_PUSH_UPDATES_PER_RUN: usize = 60;
const MAX_PUSH_CREATES_PER_RUN: usize = 40;
const MANUAL_SYNC_REMOTE_PAGES: usize = 1;
const MANUAL_SYNC_REMOTE_ISSUES: usize = 30;
const CURRENT_ASSIGNEE_SENTINEL: &str = "__current__";
const DEFAULT_SYNC_STATE_NAMES: &[&str] = &["backlog", "todo", "in review", "in progress"];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LinearState {
    connection: ConnectionState,
    sync: SyncState,
    preferences: SyncPreferences,
    mappings: Vec<TaskMapping>,
    cached_teams: Vec<LinearTeam>,
    cached_users: Vec<LinearUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ConnectionState {
    status: String,
    viewer_id: Option<String>,
    workspace_name: Option<String>,
    user_name: Option<String>,
    user_email: Option<String>,
    last_error: Option<String>,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            status: "disconnected".to_string(),
            viewer_id: None,
            workspace_name: None,
            user_name: None,
            user_email: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncState {
    last_sync_at: Option<String>,
    last_pull_cursor: Option<String>,
    last_push_cursor: Option<String>,
    error_count: u32,
    next_run_at: Option<String>,
    last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SyncPreferences {
    assignee_id: Option<String>,
    default_team_id: Option<String>,
    auto_push_new_tasks: bool,
    sync_state_names: Vec<String>,
}

impl Default for SyncPreferences {
    fn default() -> Self {
        Self {
            assignee_id: None,
            default_team_id: None,
            auto_push_new_tasks: false,
            sync_state_names: DEFAULT_SYNC_STATE_NAMES
                .iter()
                .map(|value| value.to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskMapping {
    task_id: String,
    issue_id: String,
    issue_identifier: String,
    team_id: Option<String>,
    last_local_updated_at: Option<String>,
    last_remote_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearTeam {
    id: String,
    key: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearUser {
    id: String,
    name: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetApiKeyInput {
    api_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncSummaryDto {
    pulled: usize,
    pushed: usize,
    linked: usize,
    last_sync_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionStatusDto {
    plugin_key: String,
    plugin_enabled: bool,
    connected: bool,
    status: String,
    viewer_id: Option<String>,
    workspace_name: Option<String>,
    user_name: Option<String>,
    user_email: Option<String>,
    last_sync_at: Option<String>,
    last_error: Option<String>,
    error_count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PanelSnapshotDto {
    status: ConnectionStatusDto,
    teams: Vec<LinearTeam>,
    users: Vec<LinearUser>,
    preferences: SyncPreferences,
    mapping_count: usize,
}

#[derive(Debug, Serialize)]
struct LinearGraphqlRequest<'a> {
    query: &'a str,
    variables: Value,
}

#[derive(Debug, Deserialize)]
struct LinearGraphqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<LinearGraphqlError>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearGraphqlError {
    message: String,
    extensions: Option<LinearGraphqlErrorExtensions>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearGraphqlErrorExtensions {
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewerQueryData {
    viewer: ViewerNode,
    users: UserConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewerNode {
    id: String,
    name: Option<String>,
    email: Option<String>,
    organization: Option<ViewerOrganization>,
    teams: TeamConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewerOrganization {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TeamConnection {
    nodes: Vec<LinearTeam>,
}

#[derive(Debug, Deserialize)]
struct UserConnection {
    nodes: Vec<LinearUser>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssuesQueryData {
    issues: IssuesConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssuesConnection {
    nodes: Vec<LinearIssue>,
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearIssue {
    id: String,
    identifier: String,
    title: String,
    description: Option<String>,
    priority: i64,
    updated_at: String,
    state: LinearIssueState,
    team: LinearIssueTeam,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearIssueState {
    name: Option<String>,
    #[serde(rename = "type")]
    state_type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearIssueTeam {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamStatesQueryData {
    team: Option<TeamNode>,
}

#[derive(Debug, Deserialize)]
struct TeamNode {
    states: StateConnection,
}

#[derive(Debug, Deserialize)]
struct StateConnection {
    nodes: Vec<LinearWorkflowState>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearWorkflowState {
    id: String,
    #[serde(rename = "type")]
    state_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueUpdateMutationData {
    issue_update: MutationIssuePayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueCreateMutationData {
    issue_create: MutationIssuePayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MutationIssuePayload {
    success: bool,
    issue: Option<MutationIssueNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MutationIssueNode {
    id: String,
    identifier: String,
    updated_at: String,
    team: LinearIssueTeam,
}

#[derive(Debug, Clone, Deserialize)]
struct LocalTask {
    id: String,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default, alias = "updatedAt")]
    updated_at: Option<String>,
}

#[derive(Debug, Default)]
struct SyncSummary {
    pulled: usize,
    pushed: usize,
    linked: usize,
}

#[derive(Debug, Default)]
struct TeamStateMap {
    todo_state_id: Option<String>,
    in_progress_state_id: Option<String>,
    done_state_id: Option<String>,
    cancelled_state_id: Option<String>,
}

#[plugin_fn]
pub fn plugin_init(_: String) -> FnResult<String> {
    ensure_sync_schedule().map_err(Error::msg)?;
    bootstrap_connection_status().map_err(Error::msg)?;
    Ok(r#"{"status":"ok"}"#.to_string())
}

#[plugin_fn]
pub fn on_event(input: String) -> FnResult<String> {
    let event: Value = serde_json::from_str(&input)?;
    let event_name = event["event"].as_str().unwrap_or_default();
    let key = event["payload"]["key"].as_str().unwrap_or_default();

    if event_name == "schedule:fired" && key == SYNC_SCHEDULE_KEY {
        let _ = sync_once(false);
    }

    Ok(r#"{"ok":true}"#.to_string())
}

#[plugin_fn]
pub fn tool_linear_set_api_key(input: String) -> FnResult<String> {
    let payload: SetApiKeyInput = serde_json::from_str(&input)?;
    let api_key = payload.api_key.trim();
    if api_key.is_empty() {
        return Err(Error::msg("apiKey is required").into());
    }

    let mut state = load_state().map_err(Error::msg)?;
    refresh_viewer_context(api_key, &mut state).map_err(Error::msg)?;
    peekoo::secrets::set(API_KEY_KEY, api_key)?;

    state.connection.status = "connected".to_string();
    state.connection.last_error = None;
    state.sync.last_error = None;
    state.sync.error_count = 0;
    save_state(&state).map_err(Error::msg)?;

    Ok(r#"{"ok":true}"#.to_string())
}

#[plugin_fn]
pub fn tool_linear_disconnect(_: String) -> FnResult<String> {
    let _ = peekoo::secrets::delete(API_KEY_KEY);
    let mut state = load_state().map_err(Error::msg)?;

    state.connection = ConnectionState::default();
    state.sync = SyncState::default();

    save_state(&state).map_err(Error::msg)?;
    Ok(r#"{"ok":true}"#.to_string())
}

#[plugin_fn]
pub fn tool_linear_sync_now(_: String) -> FnResult<String> {
    let summary = sync_once(true).map_err(Error::msg)?;
    Ok(serde_json::to_string(&SyncSummaryDto {
        pulled: summary.pulled,
        pushed: summary.pushed,
        linked: summary.linked,
        last_sync_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    })?)
}

#[plugin_fn]
pub fn tool_linear_set_sync_settings(input: String) -> FnResult<String> {
    let payload: Value = serde_json::from_str(&input)?;
    let mut state = load_state().map_err(Error::msg)?;

    if state.connection.viewer_id.is_none() {
        if let Some(api_key) = load_api_key().map_err(Error::msg)? {
            let _ = refresh_viewer_context(&api_key, &mut state);
        }
    }

    let assignee_raw = payload
        .get("assigneeId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let assignee = match assignee_raw.as_deref() {
        Some(CURRENT_ASSIGNEE_SENTINEL) => state.connection.viewer_id.clone(),
        Some(_) => assignee_raw.clone(),
        None => None,
    };
    let team = payload
        .get("defaultTeamId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    if payload.get("assigneeId").is_some() {
        state.preferences.assignee_id = assignee.clone();
        if assignee.is_some() {
            state.preferences.default_team_id = None;
        }
    }
    if payload.get("defaultTeamId").is_some() {
        state.preferences.default_team_id = team.clone();
        if team.is_some() {
            state.preferences.assignee_id = None;
        }
    }
    if payload.get("syncStateNames").is_some() {
        let normalized = payload
            .get("syncStateNames")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(normalize_sync_state_name)
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        state.preferences.sync_state_names = if normalized.is_empty() {
            DEFAULT_SYNC_STATE_NAMES
                .iter()
                .map(|value| value.to_string())
                .collect()
        } else {
            normalized
        };
    }
    if let Some(value) = payload.get("autoPushNewTasks").and_then(Value::as_bool) {
        state.preferences.auto_push_new_tasks = value;
    }

    save_state(&state).map_err(Error::msg)?;
    Ok(serde_json::to_string(&panel_snapshot().map_err(Error::msg)?)?)
}

#[plugin_fn]
pub fn tool_linear_get_connection_status(_: String) -> FnResult<String> {
    Ok(serde_json::to_string(&connection_status().map_err(Error::msg)?)?)
}

#[plugin_fn]
pub fn data_connection_status(_: String) -> FnResult<String> {
    Ok(serde_json::to_string(&connection_status().map_err(Error::msg)?)?)
}

#[plugin_fn]
pub fn data_panel_snapshot(_: String) -> FnResult<String> {
    Ok(serde_json::to_string(&panel_snapshot().map_err(Error::msg)?)?)
}

fn panel_snapshot() -> Result<PanelSnapshotDto, String> {
    let state = load_state()?;
    Ok(PanelSnapshotDto {
        status: ConnectionStatusDto {
            plugin_key: "linear".to_string(),
            plugin_enabled: true,
            connected: load_api_key()?.is_some(),
            status: state.connection.status,
            viewer_id: state.connection.viewer_id,
            workspace_name: state.connection.workspace_name,
            user_name: state.connection.user_name,
            user_email: state.connection.user_email,
            last_sync_at: state.sync.last_sync_at,
            last_error: state.sync.last_error.or(state.connection.last_error),
            error_count: state.sync.error_count,
        },
        teams: state.cached_teams,
        users: state.cached_users,
        preferences: state.preferences,
        mapping_count: state.mappings.len(),
    })
}

fn connection_status() -> Result<ConnectionStatusDto, String> {
    let state = load_state()?;
    Ok(ConnectionStatusDto {
        plugin_key: "linear".to_string(),
        plugin_enabled: true,
        connected: load_api_key()?.is_some(),
        status: state.connection.status,
        viewer_id: state.connection.viewer_id,
        workspace_name: state.connection.workspace_name,
        user_name: state.connection.user_name,
        user_email: state.connection.user_email,
        last_sync_at: state.sync.last_sync_at,
        last_error: state.sync.last_error.or(state.connection.last_error),
        error_count: state.sync.error_count,
    })
}

fn bootstrap_connection_status() -> Result<(), String> {
    let mut state = load_state()?;
    let connected = load_api_key()?.is_some();

    if !connected {
        state.connection.status = "disconnected".to_string();
        save_state(&state)?;
        return Ok(());
    }

    if state.connection.status == "disconnected" || state.connection.status == "error" {
        state.connection.status = "connected".to_string();
        state.connection.last_error = None;
        state.sync.last_error = None;
        save_state(&state)?;
    }

    Ok(())
}

fn sync_once(force: bool) -> Result<SyncSummary, String> {
    ensure_sync_schedule()?;

    let Some(api_key) = load_api_key()? else {
        let mut state = load_state()?;
        state.connection.status = "disconnected".to_string();
        state.connection.last_error = Some("Linear API key is not configured".to_string());
        save_state(&state)?;
        return Err("Linear API key is not configured".to_string());
    };

    let mut state = load_state()?;
    state.connection.status = "syncing".to_string();
    state.connection.last_error = None;
    save_state(&state)?;

    let sync_result = run_sync_cycle(force, &api_key, &mut state);
    match sync_result {
        Ok(summary) => {
            let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
            state.connection.status = "connected".to_string();
            state.sync.last_sync_at = Some(now.clone());
            state.sync.last_pull_cursor = Some(now.clone());
            state.sync.last_push_cursor = Some(now.clone());
            state.sync.next_run_at = Some(
                (Utc::now() + Duration::seconds(DEFAULT_REFRESH_INTERVAL_SECS as i64))
                    .to_rfc3339_opts(SecondsFormat::Secs, true),
            );
            state.sync.last_error = None;
            save_state(&state)?;
            Ok(summary)
        }
        Err(error) => {
            state.connection.status = "error".to_string();
            state.connection.last_error = Some(error.clone());
            state.sync.last_error = Some(error.clone());
            state.sync.error_count = state.sync.error_count.saturating_add(1);
            save_state(&state)?;
            Err(error)
        }
    }
}

fn run_sync_cycle(force: bool, api_key: &str, state: &mut LinearState) -> Result<SyncSummary, String> {
    if state.cached_teams.is_empty() || state.connection.workspace_name.is_none() {
        refresh_viewer_context(api_key, state)?;
    }

    let page_limit = if force {
        MANUAL_SYNC_REMOTE_PAGES
    } else {
        MAX_REMOTE_PAGES_PER_RUN
    };
    let issue_limit = if force {
        MANUAL_SYNC_REMOTE_ISSUES
    } else {
        MAX_REMOTE_ISSUES_PER_RUN
    };

    let remote_issues = fetch_remote_issues(
        api_key,
        state
            .preferences
            .assignee_id
            .as_deref()
            .or(state.connection.viewer_id.as_deref()),
        state.preferences.default_team_id.as_deref(),
        page_limit,
        issue_limit,
    )?;
    let state_filtered_issues = remote_issues
        .into_iter()
        .filter(|issue| issue_matches_sync_state(issue, &state.preferences.sync_state_names))
        .collect::<Vec<_>>();
    let filtered_issues = if force {
        state_filtered_issues
    } else if let Some(cursor) = state.sync.last_pull_cursor.as_deref() {
        state_filtered_issues
            .into_iter()
            .filter(|issue| is_after_cursor(&issue.updated_at, cursor))
            .collect()
    } else {
        state_filtered_issues
    };

    let mut local_tasks = peekoo::tasks::list::<LocalTask>(None).map_err(|e| e.to_string())?;

    let mut summary = SyncSummary::default();
    summary.pulled = pull_remote_into_local(&filtered_issues, state, &mut local_tasks)?;

    if !force {
        let push_summary = push_local_to_remote(api_key, state, &mut local_tasks)?;
        summary.pushed = push_summary.pushed;
        summary.linked = push_summary.linked;
    }

    Ok(summary)
}

fn pull_remote_into_local(
    issues: &[LinearIssue],
    state: &mut LinearState,
    local_tasks: &mut Vec<LocalTask>,
) -> Result<usize, String> {
    let mut applied = 0usize;

    for issue in issues {
        if let Some(index) = mapping_index_by_issue(&state.mappings, &issue.id) {
            let mapping = &mut state.mappings[index];
            let should_apply = mapping
                .last_remote_updated_at
                .as_deref()
                .map(|last| is_after_cursor(&issue.updated_at, last))
                .unwrap_or(true);

            if !should_apply {
                continue;
            }

            let updated = peekoo::tasks::update::<LocalTask>(json!({
                "id": mapping.task_id,
                "title": issue.title,
                "description": issue.description,
                "status": linear_state_to_task_status(&issue.state.state_type),
                "priority": linear_priority_to_task_priority(issue.priority),
            }))
            .map_err(|e| e.to_string())?;

            mapping.last_local_updated_at = Some(local_task_updated_marker(&updated));
            mapping.last_remote_updated_at = Some(issue.updated_at.clone());

            if let Some(position) = local_tasks.iter().position(|task| task.id == updated.id) {
                local_tasks[position] = updated;
            }

            applied = applied.saturating_add(1);
            continue;
        }

        let created = peekoo::tasks::create::<LocalTask>(json!({
            "title": issue.title,
            "priority": linear_priority_to_task_priority(issue.priority),
            "assignee": "user",
            "labels": vec!["linear".to_string(), format!("linear:{}", issue.identifier.to_lowercase())],
            "description": issue.description,
            "scheduled_start_at": Value::Null,
            "scheduled_end_at": Value::Null,
            "estimated_duration_min": Value::Null,
            "recurrence_rule": Value::Null,
            "recurrence_time_of_day": Value::Null,
        }))
        .map_err(|e| e.to_string())?;

        state.mappings.push(TaskMapping {
            task_id: created.id.clone(),
            issue_id: issue.id.clone(),
            issue_identifier: issue.identifier.clone(),
            team_id: Some(issue.team.id.clone()),
            last_local_updated_at: Some(local_task_updated_marker(&created)),
            last_remote_updated_at: Some(issue.updated_at.clone()),
        });

        local_tasks.push(created);
        applied = applied.saturating_add(1);
    }

    Ok(applied)
}

fn push_local_to_remote(
    api_key: &str,
    state: &mut LinearState,
    local_tasks: &mut [LocalTask],
) -> Result<SyncSummary, String> {
    let mut summary = SyncSummary::default();
    let mut state_maps = std::collections::HashMap::<String, TeamStateMap>::new();
    let mut update_budget = MAX_PUSH_UPDATES_PER_RUN;
    let mut create_budget = MAX_PUSH_CREATES_PER_RUN;

    for mapping in &mut state.mappings {
        if update_budget == 0 {
            break;
        }

        let Some(task) = local_tasks.iter().find(|task| task.id == mapping.task_id) else {
            continue;
        };

        let should_push = match (&task.updated_at, mapping.last_local_updated_at.as_deref()) {
            (Some(task_updated_at), Some(last)) => is_after_cursor(task_updated_at, last),
            _ => true,
        };

        if !should_push {
            continue;
        }

        let team_id = mapping
            .team_id
            .as_deref()
            .or(state.preferences.default_team_id.as_deref());
        let state_id = match team_id {
            Some(team_id) => {
                let entry = state_maps
                    .entry(team_id.to_string())
                    .or_insert_with(|| fetch_team_state_map(api_key, team_id).unwrap_or_default());
                choose_linear_state_id(entry, &task.status)
            }
            None => None,
        };

        let updated_issue = update_remote_issue(api_key, &mapping.issue_id, task, state_id)?;
        mapping.last_local_updated_at = Some(local_task_updated_marker(task));
        mapping.last_remote_updated_at = Some(updated_issue.updated_at.clone());
        summary.pushed = summary.pushed.saturating_add(1);
        update_budget = update_budget.saturating_sub(1);
    }

    if !state.preferences.auto_push_new_tasks {
        return Ok(summary);
    }

    let Some(default_team_id) = state.preferences.default_team_id.clone() else {
        return Ok(summary);
    };

    let state_map = state_maps
        .entry(default_team_id.clone())
        .or_insert_with(|| fetch_team_state_map(api_key, &default_team_id).unwrap_or_default());

    for task in local_tasks {
        if create_budget == 0 {
            break;
        }

        if mapping_index_by_task(&state.mappings, &task.id).is_some() {
            continue;
        }
        if task
            .labels
            .iter()
            .any(|label| label == "linear" || label.starts_with("linear:"))
        {
            continue;
        }

        let state_id = choose_linear_state_id(state_map, &task.status);
        let created_issue = create_remote_issue(api_key, &default_team_id, task, state_id)?;

        let mut labels = task.labels.clone();
        labels.push("linear".to_string());
        labels.push(format!("linear:{}", created_issue.identifier.to_lowercase()));

        let updated_local = peekoo::tasks::update::<LocalTask>(json!({
            "id": task.id,
            "labels": labels,
        }))
        .map_err(|e| e.to_string())?;

        let updated_local_marker = local_task_updated_marker(&updated_local);
        state.mappings.push(TaskMapping {
            task_id: updated_local.id,
            issue_id: created_issue.id,
            issue_identifier: created_issue.identifier,
            team_id: Some(created_issue.team.id),
            last_local_updated_at: Some(updated_local_marker),
            last_remote_updated_at: Some(created_issue.updated_at),
        });

        summary.linked = summary.linked.saturating_add(1);
        summary.pushed = summary.pushed.saturating_add(1);
        create_budget = create_budget.saturating_sub(1);
    }

    Ok(summary)
}

fn fetch_remote_issues(
    api_key: &str,
    assignee_id: Option<&str>,
    team_id: Option<&str>,
    page_limit: usize,
    issue_limit: usize,
) -> Result<Vec<LinearIssue>, String> {
    const QUERY_ASSIGNEE_WITH_TEAM: &str = r#"
      query LinearIssues($after: String, $assigneeId: ID!, $teamId: ID!) {
        issues(first: 50, after: $after, filter: { and: [{ assignee: { id: { eq: $assigneeId } } }, { team: { id: { eq: $teamId } } }] }) {
          nodes {
            id
            identifier
            title
            description
            priority
            updatedAt
            state { type name }
            team { id }
          }
          pageInfo {
            hasNextPage
            endCursor
          }
        }
      }
    "#;

    const QUERY_ASSIGNEE_ONLY: &str = r#"
      query LinearIssues($after: String, $assigneeId: ID!) {
        issues(first: 50, after: $after, filter: { assignee: { id: { eq: $assigneeId } } }) {
          nodes {
            id
            identifier
            title
            description
            priority
            updatedAt
            state { type name }
            team { id }
          }
          pageInfo {
            hasNextPage
            endCursor
          }
        }
      }
    "#;

    const QUERY_TEAM_ONLY: &str = r#"
      query LinearIssues($after: String, $teamId: ID!) {
        issues(first: 50, after: $after, filter: { team: { id: { eq: $teamId } } }) {
          nodes {
            id
            identifier
            title
            description
            priority
            updatedAt
            state { type name }
            team { id }
          }
          pageInfo {
            hasNextPage
            endCursor
          }
        }
      }
    "#;

    let mut issues = Vec::new();
    let mut after: Option<String> = None;

    for _ in 0..page_limit {
        let (query, variables) = match (assignee_id, team_id) {
            (Some(assignee_id), Some(team_id)) => (
                QUERY_ASSIGNEE_WITH_TEAM,
                json!({"after": after, "assigneeId": assignee_id, "teamId": team_id}),
            ),
            (Some(assignee_id), None) => (
                QUERY_ASSIGNEE_ONLY,
                json!({"after": after, "assigneeId": assignee_id}),
            ),
            (None, Some(team_id)) => (
                QUERY_TEAM_ONLY,
                json!({"after": after, "teamId": team_id}),
            ),
            (None, None) => {
                return Err("Linear sync target is not configured.".to_string());
            }
        };

        let data: IssuesQueryData = linear_graphql(api_key, query, variables)?;
        issues.extend(data.issues.nodes.into_iter());
        if issues.len() >= issue_limit {
            issues.truncate(issue_limit);
            break;
        }

        if !data.issues.page_info.has_next_page {
            break;
        }
        after = data.issues.page_info.end_cursor;
        if after.is_none() {
            break;
        }
    }

    Ok(issues)
}

fn update_remote_issue(
    api_key: &str,
    issue_id: &str,
    task: &LocalTask,
    state_id: Option<String>,
) -> Result<MutationIssueNode, String> {
    const MUTATION: &str = r#"
      mutation UpdateIssue($id: ID!, $input: IssueUpdateInput!) {
        issueUpdate(id: $id, input: $input) {
          success
          issue {
            id
            identifier
            updatedAt
            team { id }
          }
        }
      }
    "#;

    let mut input = json!({
        "title": task.title,
        "description": task.description,
        "priority": task_priority_to_linear_priority(&task.priority),
    });

    if let Some(state_id) = state_id {
        input["stateId"] = Value::String(state_id);
    }

    let data: IssueUpdateMutationData = linear_graphql(
        api_key,
        MUTATION,
        json!({
            "id": issue_id,
            "input": input,
        }),
    )?;

    if !data.issue_update.success {
        return Err("Linear issueUpdate returned success=false".to_string());
    }

    data.issue_update
        .issue
        .ok_or_else(|| "Linear issueUpdate missing issue payload".to_string())
}

fn create_remote_issue(
    api_key: &str,
    team_id: &str,
    task: &LocalTask,
    state_id: Option<String>,
) -> Result<MutationIssueNode, String> {
    const MUTATION: &str = r#"
      mutation CreateIssue($input: IssueCreateInput!) {
        issueCreate(input: $input) {
          success
          issue {
            id
            identifier
            updatedAt
            team { id }
          }
        }
      }
    "#;

    let mut input = json!({
        "teamId": team_id,
        "title": task.title,
        "description": task.description,
        "priority": task_priority_to_linear_priority(&task.priority),
    });

    if let Some(state_id) = state_id {
        input["stateId"] = Value::String(state_id);
    }

    let data: IssueCreateMutationData = linear_graphql(api_key, MUTATION, json!({ "input": input }))?;

    if !data.issue_create.success {
        return Err("Linear issueCreate returned success=false".to_string());
    }

    data.issue_create
        .issue
        .ok_or_else(|| "Linear issueCreate missing issue payload".to_string())
}

fn refresh_viewer_context(api_key: &str, state: &mut LinearState) -> Result<(), String> {
    const QUERY: &str = r#"
      query ViewerContext {
        viewer {
          id
          name
          email
          organization { name }
          teams {
            nodes {
              id
              key
              name
            }
          }
        }
        users(first: 100, filter: { active: { eq: true } }) {
          nodes {
            id
            name
            email
          }
        }
      }
    "#;

    let data: ViewerQueryData = linear_graphql(api_key, QUERY, json!({}))?;
    state.connection.viewer_id = Some(data.viewer.id);
    state.connection.workspace_name = data
        .viewer
        .organization
        .and_then(|organization| organization.name);
    state.connection.user_name = data.viewer.name;
    state.connection.user_email = data.viewer.email;
    state.cached_teams = data.viewer.teams.nodes;
    state.cached_users = data.users.nodes;

    if state.preferences.assignee_id.is_none() {
        state.preferences.assignee_id = state.connection.viewer_id.clone();
    }

    Ok(())
}

fn fetch_team_state_map(api_key: &str, team_id: &str) -> Result<TeamStateMap, String> {
    const QUERY: &str = r#"
      query TeamStates($teamId: ID!) {
        team(id: $teamId) {
          states {
            nodes {
              id
              type
            }
          }
        }
      }
    "#;

    let data: TeamStatesQueryData = linear_graphql(api_key, QUERY, json!({ "teamId": team_id }))?;

    let mut map = TeamStateMap::default();
    let Some(team) = data.team else {
        return Ok(map);
    };

    for state in team.states.nodes {
        match state.state_type.as_str() {
            "unstarted" | "backlog" | "triage" => {
                if map.todo_state_id.is_none() {
                    map.todo_state_id = Some(state.id);
                }
            }
            "started" => {
                if map.in_progress_state_id.is_none() {
                    map.in_progress_state_id = Some(state.id);
                }
            }
            "completed" => {
                if map.done_state_id.is_none() {
                    map.done_state_id = Some(state.id);
                }
            }
            "canceled" => {
                if map.cancelled_state_id.is_none() {
                    map.cancelled_state_id = Some(state.id);
                }
            }
            _ => {}
        }
    }

    Ok(map)
}

fn choose_linear_state_id(map: &TeamStateMap, task_status: &str) -> Option<String> {
    match task_status {
        "todo" => map.todo_state_id.clone(),
        "in_progress" => map.in_progress_state_id.clone(),
        "done" => map.done_state_id.clone(),
        "cancelled" => map
            .cancelled_state_id
            .clone()
            .or_else(|| map.done_state_id.clone()),
        _ => map.todo_state_id.clone(),
    }
}

fn linear_graphql<T: serde::de::DeserializeOwned>(
    api_key: &str,
    query: &str,
    variables: Value,
) -> Result<T, String> {
    let body = serde_json::to_string(&LinearGraphqlRequest { query, variables })
        .map_err(|e| e.to_string())?;

    let response = peekoo::http::request(peekoo::http::Request {
        method: "POST",
        url: LINEAR_GRAPHQL_URL,
        headers: vec![
            ("Authorization", api_key),
            ("Content-Type", "application/json"),
            ("Accept", "application/json"),
            ("User-Agent", "Peekoo-Desktop/0.1.0"),
        ],
        body: Some(&body),
    })
    .map_err(|e| e.to_string())?;

    if response.status == 429 {
        return Err("Linear API rate limited (HTTP 429)".to_string());
    }
    if response.status >= 400 {
        return Err(format!(
            "Linear GraphQL request failed ({}): {}",
            response.status, response.body
        ));
    }

    let parsed: LinearGraphqlResponse<T> =
        serde_json::from_str(&response.body).map_err(|e| e.to_string())?;

    if let Some(errors) = parsed.errors {
        let rate_limited = errors.iter().any(|error| {
            error
                .extensions
                .as_ref()
                .and_then(|extensions| extensions.code.as_deref())
                == Some("RATELIMITED")
        });
        let message = errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join(" | ");
        if rate_limited {
            return Err(format!("Linear API rate limited: {message}"));
        }
        return Err(format!("Linear GraphQL error: {message}"));
    }

    parsed
        .data
        .ok_or_else(|| "Linear GraphQL response missing data".to_string())
}

fn load_api_key() -> Result<Option<String>, String> {
    peekoo::secrets::get(API_KEY_KEY).map_err(|e| e.to_string())
}

fn load_state() -> Result<LinearState, String> {
    Ok(peekoo::state::get(STATE_KEY)
        .map_err(|e| e.to_string())?
        .unwrap_or_default())
}

fn save_state(state: &LinearState) -> Result<(), String> {
    peekoo::state::set(STATE_KEY, state).map_err(|e| e.to_string())
}

fn ensure_sync_schedule() -> Result<(), String> {
    peekoo::schedule::set(SYNC_SCHEDULE_KEY, DEFAULT_REFRESH_INTERVAL_SECS, true, None)
        .map_err(|e| e.to_string())
}

fn mapping_index_by_issue(mappings: &[TaskMapping], issue_id: &str) -> Option<usize> {
    mappings
        .iter()
        .position(|mapping| mapping.issue_id == issue_id)
}

fn mapping_index_by_task(mappings: &[TaskMapping], task_id: &str) -> Option<usize> {
    mappings
        .iter()
        .position(|mapping| mapping.task_id == task_id)
}

fn local_task_updated_marker(task: &LocalTask) -> String {
    task.updated_at.clone().unwrap_or_else(|| {
        Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
    })
}

fn normalize_sync_state_name(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('_', " ")
        .replace('-', " ")
}

fn issue_matches_sync_state(issue: &LinearIssue, selected_states: &[String]) -> bool {
    if selected_states.is_empty() {
        return true;
    }

    let selected = selected_states
        .iter()
        .map(|value| normalize_sync_state_name(value))
        .collect::<Vec<_>>();

    let mut candidates = Vec::new();
    if let Some(name) = issue.state.name.as_deref() {
        candidates.push(normalize_sync_state_name(name));
    }

    let fallback = match issue.state.state_type.as_str() {
        "unstarted" => "todo",
        "backlog" => "backlog",
        "triage" => "triage",
        "started" => "in progress",
        "completed" => "done",
        "canceled" => "canceled",
        other => other,
    };
    candidates.push(normalize_sync_state_name(fallback));

    if candidates.iter().any(|candidate| candidate == "in view") {
        candidates.push("in review".to_string());
    }

    selected
        .iter()
        .any(|target| candidates.iter().any(|candidate| candidate == target))
}

fn is_after_cursor(candidate: &str, cursor: &str) -> bool {
    parse_rfc3339(candidate)
        .zip(parse_rfc3339(cursor))
        .map(|(candidate, cursor)| candidate > cursor)
        .unwrap_or_else(|| candidate > cursor)
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn linear_state_to_task_status(state_type: &str) -> &'static str {
    match state_type {
        "started" => "in_progress",
        "completed" => "done",
        "canceled" => "cancelled",
        _ => "todo",
    }
}

fn linear_priority_to_task_priority(priority: i64) -> &'static str {
    match priority {
        1 | 2 => "high",
        4 => "low",
        _ => "medium",
    }
}

fn task_priority_to_linear_priority(priority: &str) -> i64 {
    match priority {
        "high" => 2,
        "low" => 4,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        linear_priority_to_task_priority, linear_state_to_task_status,
        task_priority_to_linear_priority,
    };

    #[test]
    fn maps_linear_state_to_task_status() {
        assert_eq!(linear_state_to_task_status("started"), "in_progress");
        assert_eq!(linear_state_to_task_status("completed"), "done");
        assert_eq!(linear_state_to_task_status("canceled"), "cancelled");
        assert_eq!(linear_state_to_task_status("triage"), "todo");
    }

    #[test]
    fn maps_priorities_bidirectionally() {
        assert_eq!(linear_priority_to_task_priority(2), "high");
        assert_eq!(linear_priority_to_task_priority(4), "low");
        assert_eq!(linear_priority_to_task_priority(3), "medium");

        assert_eq!(task_priority_to_linear_priority("high"), 2);
        assert_eq!(task_priority_to_linear_priority("medium"), 3);
        assert_eq!(task_priority_to_linear_priority("low"), 4);
    }
}
