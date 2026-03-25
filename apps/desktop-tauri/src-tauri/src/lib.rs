// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use peekoo_agent_app::{
    AgentApplication, AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto,
    LastSessionDto, OauthCancelResponse, OauthStartResponse, OauthStatusRequest,
    OauthStatusResponse, PluginConfigFieldDto, PluginNotificationDto, PluginPanelDto,
    PluginSummaryDto, PomodoroCycleDto, PomodoroSettingsInput, PomodoroStatusDto, ProviderAuthDto,
    ProviderConfigDto, ProviderRequest, SetApiKeyRequest, SetProviderConfigRequest, SpriteInfo,
    StorePluginDto, TaskDto, TaskEventDto,
};
use serde::Serialize;
use std::env;
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::process::Command;
use std::time::Duration;
use tauri::{
    AppHandle, Emitter, LogicalSize, LogicalUnit, Manager, PixelUnit, State, Window,
    WindowSizeConstraints,
    image::Image,
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
};
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_shell::ShellExt;
// ============================================================================
// Agent State — lazily initialized on first prompt
// ============================================================================

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ICON_ID: &str = "main-tray";
const TRAY_TOGGLE_MENU_ID: &str = "toggle_visible";
const TRAY_SETTINGS_MENU_ID: &str = "open_settings";
const TRAY_ABOUT_MENU_ID: &str = "open_about";
const TRAY_QUIT_MENU_ID: &str = "quit";
const TRAY_TOOLTIP: &str = "Peekoo";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayMenuAction {
    ToggleVisible,
    OpenSettings,
    OpenAbout,
    Quit,
}

fn tray_menu_action(menu_id: &str) -> Option<TrayMenuAction> {
    match menu_id {
        TRAY_TOGGLE_MENU_ID => Some(TrayMenuAction::ToggleVisible),
        TRAY_SETTINGS_MENU_ID => Some(TrayMenuAction::OpenSettings),
        TRAY_ABOUT_MENU_ID => Some(TrayMenuAction::OpenAbout),
        TRAY_QUIT_MENU_ID => Some(TrayMenuAction::Quit),
        _ => None,
    }
}

struct AgentState {
    app: AgentApplication,
}

impl AgentState {
    fn new() -> Self {
        Self {
            app: AgentApplication::new()
                .unwrap_or_else(|e| panic!("Failed to initialize agent application: {e}")),
        }
    }
}

#[derive(Serialize)]
struct AgentResponse {
    response: String,
}

#[derive(Serialize)]
struct ModelInfo {
    provider: String,
    model: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainWindowVisibilityAction {
    Hide,
    ShowAndFocus,
}

fn next_main_window_visibility_action(is_visible: bool) -> MainWindowVisibilityAction {
    if is_visible {
        MainWindowVisibilityAction::Hide
    } else {
        MainWindowVisibilityAction::ShowAndFocus
    }
}

fn apply_main_window_visibility_action(app: &AppHandle, action: MainWindowVisibilityAction) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        match action {
            MainWindowVisibilityAction::Hide => {
                let _ = window.hide();
            }
            MainWindowVisibilityAction::ShowAndFocus => {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }
}

fn toggle_main_window_visibility(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let is_visible = window.is_visible().unwrap_or(true);
        let action = next_main_window_visibility_action(is_visible);
        apply_main_window_visibility_action(app, action);
    }
}

fn handle_tray_menu_event(app: &AppHandle, menu_id: &str) {
    match tray_menu_action(menu_id) {
        Some(TrayMenuAction::ToggleVisible) => toggle_main_window_visibility(app),
        Some(TrayMenuAction::OpenSettings) => {
            apply_main_window_visibility_action(app, MainWindowVisibilityAction::ShowAndFocus);
            let _ = app.emit_to(MAIN_WINDOW_LABEL, "open-settings", ());
        }
        Some(TrayMenuAction::OpenAbout) => {
            apply_main_window_visibility_action(app, MainWindowVisibilityAction::ShowAndFocus);
            let _ = app.emit_to(MAIN_WINDOW_LABEL, "open-about", ());
        }
        Some(TrayMenuAction::Quit) => app.exit(0),
        None => {}
    }
}

fn handle_tray_icon_event(app: &AppHandle, event: &TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Down,
        ..
    } = event
    {
        toggle_main_window_visibility(app);
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Resize the sprite window from Rust and keep tight size constraints in sync.
/// This is more reliable on Linux / Wayland compositors than resizing a non-resizable window.
/// `delta_top` shifts the window vertically in logical pixels (positive = move up, negative = move down).
/// `delta_left` shifts the window horizontally in logical pixels (positive = move left, negative = move right).
#[tauri::command]
async fn resize_sprite_window(
    width: f64,
    height: f64,
    delta_left: f64,
    delta_top: f64,
    window: Window,
) -> Result<(), String> {
    window
        .set_resizable(true)
        .map_err(|e| format!("set resizable error: {e}"))?;

    let constraints = WindowSizeConstraints {
        min_width: Some(PixelUnit::Logical(LogicalUnit(width))),
        min_height: Some(PixelUnit::Logical(LogicalUnit(height))),
        max_width: Some(PixelUnit::Logical(LogicalUnit(width))),
        max_height: Some(PixelUnit::Logical(LogicalUnit(height))),
    };

    window
        .set_size_constraints(constraints)
        .map_err(|e| format!("set size constraints error: {e}"))?;

    if delta_top.abs() > 0.5 || delta_left.abs() > 0.5 {
        let pos = window
            .outer_position()
            .map_err(|e| format!("get position error: {e}"))?;
        let scale = window
            .scale_factor()
            .map_err(|e| format!("scale error: {e}"))?;
        let logical_x = pos.x as f64 / scale - delta_left;
        let logical_y = pos.y as f64 / scale - delta_top;
        let physical_x = (logical_x * scale).round() as i32;
        let physical_y = (logical_y * scale).round() as i32;
        window
            .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: physical_x,
                y: physical_y,
            }))
            .map_err(|e| format!("set position error: {e}"))?;
    }

    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| format!("resize error: {e}"))?;

    window
        .set_resizable(false)
        .map_err(|e| format!("restore resizable error: {e}"))
}

#[tauri::command]
async fn greet(name: String) -> Result<String, String> {
    Ok(format!(
        "Hello, {}! This is Peekoo Desktop (Tauri Version)",
        name
    ))
}

#[tauri::command]
async fn get_sprite_state() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "mood": "idle",
        "message": "Welcome to Peekoo! Your AI desktop sprite is ready to help you!",
        "animation": "idle"
    }))
}

#[tauri::command]
async fn agent_prompt(
    message: String,
    window: Window,
    state: State<'_, AgentState>,
) -> Result<AgentResponse, String> {
    let reply = state
        .app
        .prompt_streaming(&message, move |event| {
            let _ = window.emit("agent-event", event);
        })
        .await?;
    Ok(AgentResponse { response: reply })
}

#[tauri::command]
async fn agent_settings_get(state: State<'_, AgentState>) -> Result<AgentSettingsDto, String> {
    state.app.get_settings()
}

#[tauri::command]
async fn agent_settings_update(
    patch: AgentSettingsPatchDto,
    state: State<'_, AgentState>,
) -> Result<AgentSettingsDto, String> {
    state.app.update_settings(patch)
}

#[tauri::command]
async fn agent_settings_catalog(
    state: State<'_, AgentState>,
) -> Result<AgentSettingsCatalogDto, String> {
    state.app.settings_catalog()
}

#[tauri::command]
async fn agent_provider_auth_set_api_key(
    req: SetApiKeyRequest,
    state: State<'_, AgentState>,
) -> Result<ProviderAuthDto, String> {
    state.app.set_provider_api_key(req)
}

#[tauri::command]
async fn agent_provider_auth_clear(
    req: ProviderRequest,
    state: State<'_, AgentState>,
) -> Result<ProviderAuthDto, String> {
    state.app.clear_provider_auth(req)
}

#[tauri::command]
async fn agent_provider_config_set(
    req: SetProviderConfigRequest,
    state: State<'_, AgentState>,
) -> Result<ProviderConfigDto, String> {
    state.app.set_provider_config(req)
}

#[tauri::command]
async fn agent_oauth_start(
    req: ProviderRequest,
    state: State<'_, AgentState>,
) -> Result<OauthStartResponse, String> {
    state.app.oauth_start(req)
}

#[tauri::command]
async fn agent_oauth_status(
    req: OauthStatusRequest,
    state: State<'_, AgentState>,
) -> Result<OauthStatusResponse, String> {
    state.app.oauth_status(req).await
}

#[tauri::command]
async fn agent_oauth_cancel(
    req: OauthStatusRequest,
    state: State<'_, AgentState>,
) -> Result<OauthCancelResponse, String> {
    state.app.oauth_cancel(req)
}

#[tauri::command]
async fn system_open_url(url: String, app: AppHandle) -> Result<(), String> {
    #[allow(deprecated)]
    app.shell()
        .open(&url, None)
        .map(|_| ())
        .map_err(|e| format!("Open URL error: {e}"))
}

// ── Global app settings ─────────────────────────────────────────────────

#[tauri::command]
async fn app_settings_get(
    state: State<'_, AgentState>,
) -> Result<std::collections::HashMap<String, String>, String> {
    state.app.get_app_settings()
}

#[tauri::command]
async fn app_settings_set(
    key: String,
    value: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.set_app_setting(&key, &value)
}

#[tauri::command]
async fn app_settings_list_sprites(
    state: State<'_, AgentState>,
) -> Result<Vec<SpriteInfo>, String> {
    Ok(state.app.list_sprites())
}
#[tauri::command]
async fn agent_set_model(
    provider: String,
    model: String,
    state: State<'_, AgentState>,
) -> Result<ModelInfo, String> {
    state.app.set_model(&provider, &model).await?;
    Ok(ModelInfo { provider, model })
}

#[tauri::command]
async fn agent_get_model(state: State<'_, AgentState>) -> Result<ModelInfo, String> {
    let (provider, model) = state.app.get_model()?;
    Ok(ModelInfo { provider, model })
}

#[tauri::command]
async fn chat_get_last_session(
    state: State<'_, AgentState>,
) -> Result<Option<LastSessionDto>, String> {
    state.app.get_last_session().await
}

#[tauri::command]
async fn chat_new_session(state: State<'_, AgentState>) -> Result<(), String> {
    state.app.new_session()
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn create_task(
    title: String,
    priority: String,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
    description: Option<String>,
    scheduled_start_at: Option<String>,
    scheduled_end_at: Option<String>,
    estimated_duration_min: Option<u32>,
    recurrence_rule: Option<String>,
    recurrence_time_of_day: Option<String>,
    state: State<'_, AgentState>,
) -> Result<TaskDto, String> {
    let assignee = assignee.as_deref().unwrap_or("user");
    let labels = labels.as_deref().unwrap_or(&[]);
    state.app.create_task(
        &title,
        &priority,
        assignee,
        labels,
        description.as_deref(),
        scheduled_start_at.as_deref(),
        scheduled_end_at.as_deref(),
        estimated_duration_min,
        recurrence_rule.as_deref(),
        recurrence_time_of_day.as_deref(),
    )
}

#[tauri::command]
async fn create_task_from_text(
    text: String,
    state: State<'_, AgentState>,
) -> Result<TaskDto, String> {
    state.app.create_task_from_text(&text)
}

#[tauri::command]
async fn list_tasks(state: State<'_, AgentState>) -> Result<Vec<TaskDto>, String> {
    state.app.list_tasks()
}

#[tauri::command]
#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
async fn update_task(
    id: String,
    title: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
    description: Option<String>,
    scheduled_start_at: Option<String>,
    scheduled_end_at: Option<String>,
    estimated_duration_min: Option<Option<u32>>,
    recurrenceRule: Option<Option<String>>,
    recurrenceTimeOfDay: Option<Option<String>>,
    state: State<'_, AgentState>,
) -> Result<TaskDto, String> {
    println!("[update_task tauri] recurrenceRule = {:?}", recurrenceRule);
    let recurrence_rule = recurrenceRule;
    let recurrence_time_of_day = recurrenceTimeOfDay;
    state.app.update_task(
        &id,
        title.as_deref(),
        priority.as_deref(),
        status.as_deref(),
        assignee.as_deref(),
        labels.as_deref(),
        description.as_deref(),
        scheduled_start_at.as_deref(),
        scheduled_end_at.as_deref(),
        estimated_duration_min,
        recurrence_rule.as_ref().map(|o| o.as_deref()),
        recurrence_time_of_day.as_ref().map(|o| o.as_deref()),
    )
}

#[tauri::command]
async fn delete_task(id: String, state: State<'_, AgentState>) -> Result<(), String> {
    state.app.delete_task(&id)
}

#[tauri::command]
async fn toggle_task(id: String, state: State<'_, AgentState>) -> Result<TaskDto, String> {
    state.app.toggle_task(&id)
}

#[tauri::command]
#[allow(non_snake_case)]
async fn get_task_activity(
    taskId: String,
    limit: Option<u32>,
    state: State<'_, AgentState>,
) -> Result<Vec<TaskEventDto>, String> {
    state.app.get_task_activity(&taskId, limit.unwrap_or(50))
}

#[tauri::command]
async fn task_list_events(
    limit: Option<i64>,
    state: State<'_, AgentState>,
) -> Result<Vec<TaskEventDto>, String> {
    state.app.list_task_events(limit.unwrap_or(50))
}

#[tauri::command]
#[allow(non_snake_case)]
async fn add_task_comment(
    taskId: String,
    text: String,
    author: String,
    state: State<'_, AgentState>,
) -> Result<TaskEventDto, String> {
    state.app.add_task_comment(&taskId, &text, &author)
}

#[tauri::command]
#[allow(non_snake_case)]
async fn delete_task_event(eventId: String, state: State<'_, AgentState>) -> Result<(), String> {
    state.app.delete_task_event(&eventId)
}

#[tauri::command]
async fn pomodoro_get_status(
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroStatusDto, String> {
    let status = state.app.pomodoro_status()?;
    flush_plugin_notifications(&app, &state)?;
    Ok(status)
}

#[tauri::command]
async fn pomodoro_set_settings(
    work_minutes: u32,
    break_minutes: u32,
    enable_memo: bool,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroStatusDto, String> {
    let status = state.app.pomodoro_set_settings(PomodoroSettingsInput {
        work_minutes,
        break_minutes,
        enable_memo,
    })?;
    flush_plugin_notifications(&app, &state)?;
    Ok(status)
}

#[tauri::command]
async fn pomodoro_start(
    mode: String,
    minutes: u32,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroStatusDto, String> {
    let session = state.app.start_pomodoro(&mode, minutes)?;
    flush_plugin_notifications(&app, &state)?;
    Ok(session)
}

#[tauri::command]
async fn pomodoro_pause(
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroStatusDto, String> {
    let session = state.app.pause_pomodoro()?;
    flush_plugin_notifications(&app, &state)?;
    Ok(session)
}

#[tauri::command]
async fn pomodoro_resume(
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroStatusDto, String> {
    let session = state.app.resume_pomodoro()?;
    flush_plugin_notifications(&app, &state)?;
    Ok(session)
}

#[tauri::command]
async fn pomodoro_finish(
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroStatusDto, String> {
    let session = state.app.finish_pomodoro()?;
    flush_plugin_notifications(&app, &state)?;
    Ok(session)
}

#[tauri::command]
async fn pomodoro_switch_mode(
    mode: String,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroStatusDto, String> {
    let status = state.app.switch_pomodoro_mode(&mode)?;
    flush_plugin_notifications(&app, &state)?;
    Ok(status)
}

#[tauri::command]
async fn pomodoro_history(
    limit: usize,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<Vec<PomodoroCycleDto>, String> {
    let history = state.app.pomodoro_history(limit)?;
    flush_plugin_notifications(&app, &state)?;
    Ok(history)
}

#[tauri::command]
async fn plugins_list(state: State<'_, AgentState>) -> Result<Vec<PluginSummaryDto>, String> {
    state.app.list_plugins()
}

#[tauri::command]
async fn plugin_panels_list(state: State<'_, AgentState>) -> Result<Vec<PluginPanelDto>, String> {
    state.app.list_plugin_panels()
}

#[tauri::command]
async fn plugin_call_tool(
    tool_name: String,
    args_json: String,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<String, String> {
    let result = state.app.call_plugin_tool(&tool_name, &args_json)?;
    flush_plugin_notifications(&app, &state)?;

    Ok(result)
}

#[tauri::command]
async fn plugin_call_panel_tool(
    plugin_key: String,
    tool_name: String,
    args_json: String,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<String, String> {
    let result = state
        .app
        .call_plugin_panel_tool(&plugin_key, &tool_name, &args_json)?;
    flush_plugin_notifications(&app, &state)?;
    Ok(result)
}

#[tauri::command]
async fn plugin_query_data(
    plugin_key: String,
    provider_name: String,
    state: State<'_, AgentState>,
) -> Result<String, String> {
    state.app.query_plugin_data(&plugin_key, &provider_name)
}

#[tauri::command]
async fn plugin_panel_html(label: String, state: State<'_, AgentState>) -> Result<String, String> {
    state.app.plugin_panel_html(&label)
}

#[tauri::command]
async fn plugin_dispatch_event(
    event_name: String,
    payload_json: String,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<(), String> {
    state
        .app
        .dispatch_plugin_event(&event_name, &payload_json)?;
    flush_plugin_notifications(&app, &state)
}

#[tauri::command]
async fn plugin_config_schema(
    plugin_key: String,
    state: State<'_, AgentState>,
) -> Result<Vec<PluginConfigFieldDto>, String> {
    state.app.plugin_config_schema(&plugin_key)
}

#[tauri::command]
async fn plugin_config_get(
    plugin_key: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    state.app.plugin_config_values(&plugin_key)
}

#[tauri::command]
async fn plugin_config_set(
    plugin_key: String,
    key: String,
    value: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.plugin_config_set(&plugin_key, &key, value)
}

#[tauri::command]
async fn dnd_get(state: State<'_, AgentState>) -> Result<bool, String> {
    Ok(state.app.is_dnd())
}

#[tauri::command]
async fn dnd_set(active: bool, state: State<'_, AgentState>) -> Result<(), String> {
    state.app.set_dnd(active);
    Ok(())
}

/// Signal from the UI that it has mounted and is listening for events.
///
/// This unblocks the background flush loop so it can start emitting
/// peek-badge updates.  Without this gate, badges pushed during plugin
/// initialisation would be consumed and discarded before the frontend
/// had registered its event listeners.
#[tauri::command]
async fn ui_ready(state: State<'_, AgentState>) -> Result<(), String> {
    state.app.mark_ui_ready();
    Ok(())
}

#[tauri::command]
async fn plugin_enable(
    plugin_key: String,
    window: Window,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.enable_plugin(&plugin_key)?;
    let _ = window.emit("plugins-changed", ());
    Ok(())
}

#[tauri::command]
async fn plugin_disable(
    plugin_key: String,
    window: Window,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.disable_plugin(&plugin_key)?;
    let _ = window.emit("plugins-changed", ());
    Ok(())
}

#[tauri::command]
async fn plugin_store_catalog(state: State<'_, AgentState>) -> Result<Vec<StorePluginDto>, String> {
    state.app.store_catalog()
}

#[tauri::command]
async fn plugin_store_install(
    plugin_key: String,
    window: Window,
    state: State<'_, AgentState>,
) -> Result<StorePluginDto, String> {
    let result = state.app.store_install(&plugin_key)?;
    let _ = window.emit("plugins-changed", ());
    Ok(result)
}

#[tauri::command]
async fn plugin_store_update(
    plugin_key: String,
    window: Window,
    state: State<'_, AgentState>,
) -> Result<StorePluginDto, String> {
    let result = state.app.store_update(&plugin_key)?;
    let _ = window.emit("plugins-changed", ());
    Ok(result)
}

#[tauri::command]
async fn plugin_store_uninstall(
    plugin_key: String,
    window: Window,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.store_uninstall(&plugin_key)?;
    let _ = window.emit("plugins-changed", ());
    Ok(())
}

// ============================================================================
// WebView2 data directory (Windows)
// ============================================================================

/// Try each candidate directory in order, returning the first one that can be
/// created and written to successfully. The `try_create` callback is responsible
/// for creating the directory (or simulating creation in tests).
#[cfg(any(target_os = "windows", test))]
fn can_write_to_dir(path: &std::path::Path) -> std::io::Result<()> {
    let test_file = path.join(".peekoo-write-test");
    std::fs::write(&test_file, b"test")?;
    let _ = std::fs::remove_file(&test_file);
    Ok(())
}

#[cfg(any(target_os = "windows", test))]
fn resolve_webview2_data_dir_with_write_check<F, W>(
    candidates: &[(&str, PathBuf)],
    mut try_create: F,
    mut can_write: W,
) -> Option<PathBuf>
where
    F: FnMut(&std::path::Path) -> std::io::Result<()>,
    W: FnMut(&std::path::Path) -> std::io::Result<()>,
{
    for (label, path) in candidates {
        match try_create(path) {
            Ok(()) => match can_write(path) {
                Ok(_) => {
                    eprintln!(
                        "info: WebView2 data folder set to ({label}): {}",
                        path.display()
                    );
                    return Some(path.clone());
                }
                Err(e) => {
                    eprintln!(
                        "info: {label} WebView2 path not writable ({:?}): {e}",
                        path.display()
                    );
                }
            },
            Err(e) => {
                eprintln!(
                    "info: failed to use {label} WebView2 path ({:?}): {e}",
                    path.display()
                );
            }
        }
    }
    None
}

#[cfg(any(target_os = "windows", test))]
fn resolve_webview2_data_dir<F>(candidates: &[(&str, PathBuf)], try_create: F) -> Option<PathBuf>
where
    F: FnMut(&std::path::Path) -> std::io::Result<()>,
{
    resolve_webview2_data_dir_with_write_check(candidates, try_create, can_write_to_dir)
}

/// Build the ordered list of candidate directories for WebView2 user data.
#[cfg(target_os = "windows")]
fn webview2_candidate_dirs() -> Vec<(&'static str, PathBuf)> {
    let mut v = Vec::new();
    // Primary: %LOCALAPPDATA%\Peekoo\WebView2
    if let Some(mut p) = dirs::data_local_dir() {
        p.push("Peekoo");
        p.push("WebView2");
        v.push(("primary", p));
    }
    // Fallback: %USERPROFILE%\.peekoo\webview2
    if let Some(mut p) = dirs::home_dir() {
        p.push(".peekoo");
        p.push("webview2");
        v.push(("home", p));
    }
    // Last resort: %TEMP%\peekoo-webview-data
    v.push(("temp", std::env::temp_dir().join("peekoo-webview-data")));
    v
}

// ============================================================================
// App Entry
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "windows")]
    {
        if std::env::var("WEBVIEW2_USER_DATA_FOLDER").is_err() {
            let candidates = webview2_candidate_dirs();
            if let Some(dir) =
                resolve_webview2_data_dir(&candidates, |p| std::fs::create_dir_all(p))
            {
                // SAFETY: Called at the start of `run()` before `tauri::Builder`
                // is constructed, so no other threads are running yet.
                unsafe { std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", dir) };
            }
        }
    }

    let default_level = env::var("RUST_LOG")
        .ok()
        .and_then(|v| v.parse::<log::LevelFilter>().ok())
        .unwrap_or({
            if cfg!(debug_assertions) {
                log::LevelFilter::Info
            } else {
                log::LevelFilter::Error
            }
        });

    let file_target = if cfg!(debug_assertions) {
        let log_dir = env::var("PEEKOO_PROJECT_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        Target::new(TargetKind::Folder {
            path: log_dir,
            file_name: None,
        })
    } else {
        Target::new(TargetKind::LogDir { file_name: None })
    };

    let agent_state = AgentState::new();

    tauri::Builder::default()
        .manage(agent_state)
        .setup(|app| {
            let tray_menu = MenuBuilder::new(app)
                .text(TRAY_TOGGLE_MENU_ID, "Show/Hide Pet")
                .text(TRAY_SETTINGS_MENU_ID, "Settings")
                .text(TRAY_ABOUT_MENU_ID, "About Peekoo")
                .separator()
                .text(TRAY_QUIT_MENU_ID, "Quit Peekoo")
                .build()?;

            let mut tray_builder = tauri::tray::TrayIconBuilder::with_id(TRAY_ICON_ID)
                .menu(&tray_menu)
                .tooltip(TRAY_TOOLTIP)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| handle_tray_menu_event(app, event.id().as_ref()))
                .on_tray_icon_event(|tray, event| {
                    handle_tray_icon_event(tray.app_handle(), &event)
                });

            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            } else {
                // Fallback tray icon to ensure we always have a visible icon even
                // when no bundled window icon is configured (common in dev).
                //
                // This uses a small 16x16 RGBA image with a simple colored square.
                const SIZE: u32 = 16;
                const PIXELS: usize = (SIZE * SIZE * 4) as usize;
                let mut rgba = Vec::with_capacity(PIXELS);
                // Solid teal-like color with full opacity.
                for _ in 0..(SIZE * SIZE) {
                    rgba.push(0x1a); // R
                    rgba.push(0xa3); // G
                    rgba.push(0xff); // B
                    rgba.push(0xff); // A
                }
                let image = Image::new_owned(rgba, SIZE, SIZE);
                tray_builder = tray_builder.icon(image);
            }

            #[cfg(target_os = "macos")]
            {
                tray_builder = tray_builder.icon_as_template(true);
            }

            let _ = tray_builder.build(app)?;

            let state = app.state::<AgentState>();
            state.app.start_plugin_runtime();

            let app_handle = app.handle().clone();
            let shutdown = state.app.shutdown_token();
            tauri::async_runtime::spawn(async move {
                let mut consecutive_errors: u32 = 0;
                loop {
                    let delay = if consecutive_errors > 0 {
                        Duration::from_millis(250 * u64::from(consecutive_errors.min(16)))
                    } else {
                        Duration::from_millis(250)
                    };

                    tokio::select! {
                        _ = shutdown.cancelled() => break,
                        _ = tokio::time::sleep(delay) => {}
                    }

                    let state = app_handle.state::<AgentState>();
                    match flush_plugin_notifications(&app_handle, &state) {
                        Ok(()) => {
                            consecutive_errors = 0;
                        }
                        Err(err) => {
                            consecutive_errors = consecutive_errors.saturating_add(1);
                            tracing::warn!(
                                consecutive_errors,
                                "Background notification flush error: {err}"
                            );
                        }
                    }
                }

                tracing::info!("Background notification loop stopped");
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == MAIN_WINDOW_LABEL {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(default_level)
                .targets([file_target, Target::new(TargetKind::Stdout)])
                .max_file_size(5_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            ui_ready,
            resize_sprite_window,
            greet,
            get_sprite_state,
            agent_prompt,
            agent_set_model,
            agent_get_model,
            chat_get_last_session,
            chat_new_session,
            agent_settings_get,
            agent_settings_update,
            agent_settings_catalog,
            agent_provider_auth_set_api_key,
            agent_provider_auth_clear,
            agent_provider_config_set,
            agent_oauth_start,
            agent_oauth_status,
            agent_oauth_cancel,
            system_open_url,
            app_settings_get,
            app_settings_set,
            app_settings_list_sprites,
            create_task,
            create_task_from_text,
            list_tasks,
            update_task,
            delete_task,
            toggle_task,
            get_task_activity,
            task_list_events,
            add_task_comment,
            delete_task_event,
            pomodoro_get_status,
            pomodoro_set_settings,
            pomodoro_start,
            pomodoro_pause,
            pomodoro_resume,
            pomodoro_finish,
            pomodoro_switch_mode,
            pomodoro_history,
            plugins_list,
            plugin_panels_list,
            plugin_call_tool,
            plugin_call_panel_tool,
            plugin_query_data,
            plugin_panel_html,
            plugin_dispatch_event,
            plugin_config_schema,
            plugin_config_get,
            plugin_config_set,
            dnd_get,
            dnd_set,
            plugin_enable,
            plugin_disable,
            plugin_store_catalog,
            plugin_store_install,
            plugin_store_update,
            plugin_store_uninstall,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn show_plugin_notification(
    app: &AppHandle,
    notification: &PluginNotificationDto,
) -> Result<(), String> {
    match app
        .notification()
        .builder()
        .title(&notification.title)
        .body(&notification.body)
        .show()
    {
        Ok(()) => Ok(()),
        Err(err) => {
            #[cfg(target_os = "linux")]
            {
                if send_linux_notification_fallback(notification).is_ok() {
                    return Ok(());
                }
            }

            Err(format!("Notification error: {err}"))
        }
    }
}

fn flush_plugin_notifications(app: &AppHandle, state: &AgentState) -> Result<(), String> {
    for notification in state.app.drain_plugin_notifications() {
        show_plugin_notification(app, &notification)?;
        app.emit_to("main", "sprite:bubble", &notification)
            .map_err(|e| format!("Sprite bubble emit error: {e}"))?;
    }

    flush_peek_badges(app, state)?;
    flush_mood_reactions(app, state)?;
    Ok(())
}

fn flush_mood_reactions(app: &AppHandle, state: &AgentState) -> Result<(), String> {
    for reaction in state.app.drain_mood_reactions() {
        app.emit_to(
            "main",
            "pet:react",
            &PetReactionPayload {
                trigger: reaction.trigger,
                sticky: Some(reaction.sticky),
            },
        )
        .map_err(|e| format!("Mood reaction emit error: {e}"))?;
    }
    Ok(())
}

#[derive(Serialize)]
struct PetReactionPayload {
    trigger: String,
    sticky: Option<bool>,
}

fn flush_peek_badges(app: &AppHandle, state: &AgentState) -> Result<(), String> {
    if let Some(badges) = state.app.take_peek_badges_if_changed() {
        app.emit_to("main", "sprite:peek-badges", &badges)
            .map_err(|e| format!("Peek badge emit error: {e}"))?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn send_linux_notification_fallback(notification: &PluginNotificationDto) -> Result<(), String> {
    let status = Command::new("notify-send")
        .arg(&notification.title)
        .arg(&notification.body)
        .status()
        .map_err(|e| format!("notify-send launch error: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("notify-send exited with status {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MainWindowVisibilityAction, TrayMenuAction, next_main_window_visibility_action,
        tray_menu_action,
    };
    use super::{resolve_webview2_data_dir, resolve_webview2_data_dir_with_write_check};
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn visible_window_hides_on_toggle() {
        assert_eq!(
            next_main_window_visibility_action(true),
            MainWindowVisibilityAction::Hide
        );
    }

    #[test]
    fn hidden_window_shows_and_focuses_on_toggle() {
        assert_eq!(
            next_main_window_visibility_action(false),
            MainWindowVisibilityAction::ShowAndFocus
        );
    }

    #[test]
    fn tray_menu_maps_settings_action() {
        assert_eq!(
            tray_menu_action("open_settings"),
            Some(TrayMenuAction::OpenSettings)
        );
    }

    #[test]
    fn tray_menu_maps_about_action() {
        assert_eq!(
            tray_menu_action("open_about"),
            Some(TrayMenuAction::OpenAbout)
        );
    }

    #[test]
    fn tray_menu_rejects_unknown_ids() {
        assert_eq!(tray_menu_action("unknown"), None);
    }

    // -- WebView2 data directory fallback tests --

    #[test]
    fn webview2_picks_first_writable_candidate() {
        let candidates: Vec<(&str, PathBuf)> = vec![
            ("primary", PathBuf::from("/fake/primary")),
            ("home", PathBuf::from("/fake/home")),
            ("temp", PathBuf::from("/fake/temp")),
        ];

        let result =
            resolve_webview2_data_dir_with_write_check(&candidates, |_| Ok(()), |_| Ok(()));

        assert_eq!(result, Some(PathBuf::from("/fake/primary")));
    }

    #[test]
    fn webview2_skips_inaccessible_picks_next() {
        let candidates: Vec<(&str, PathBuf)> = vec![
            ("primary", PathBuf::from("/fake/primary")),
            ("home", PathBuf::from("/fake/home")),
            ("temp", PathBuf::from("/fake/temp")),
        ];

        let result = resolve_webview2_data_dir_with_write_check(
            &candidates,
            |p| {
                if p == std::path::Path::new("/fake/primary") {
                    Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Access Denied",
                    ))
                } else {
                    Ok(())
                }
            },
            |_| Ok(()),
        );

        assert_eq!(result, Some(PathBuf::from("/fake/home")));
    }

    #[test]
    fn webview2_falls_through_to_last_resort() {
        let candidates: Vec<(&str, PathBuf)> = vec![
            ("primary", PathBuf::from("/fake/primary")),
            ("home", PathBuf::from("/fake/home")),
            ("temp", PathBuf::from("/fake/temp")),
        ];

        let result = resolve_webview2_data_dir_with_write_check(
            &candidates,
            |p| {
                if p == std::path::Path::new("/fake/temp") {
                    Ok(())
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Access Denied",
                    ))
                }
            },
            |_| Ok(()),
        );

        assert_eq!(result, Some(PathBuf::from("/fake/temp")));
    }

    #[test]
    fn webview2_returns_none_when_all_fail() {
        let candidates: Vec<(&str, PathBuf)> = vec![
            ("primary", PathBuf::from("/fake/primary")),
            ("home", PathBuf::from("/fake/home")),
            ("temp", PathBuf::from("/fake/temp")),
        ];

        let result = resolve_webview2_data_dir_with_write_check(
            &candidates,
            |_| {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Access Denied",
                ))
            },
            |_| Ok(()),
        );

        assert_eq!(result, None);
    }

    #[test]
    fn webview2_returns_none_for_empty_candidates() {
        let candidates: Vec<(&str, PathBuf)> = vec![];

        let result = resolve_webview2_data_dir(&candidates, |_| Ok(()));

        assert_eq!(result, None);
    }

    #[test]
    fn webview2_stops_after_first_success() {
        let candidates: Vec<(&str, PathBuf)> = vec![
            ("primary", PathBuf::from("/fake/primary")),
            ("home", PathBuf::from("/fake/home")),
            ("temp", PathBuf::from("/fake/temp")),
        ];

        let mut attempts = Vec::new();
        let result = resolve_webview2_data_dir_with_write_check(
            &candidates,
            |p| {
                attempts.push(p.to_path_buf());
                Ok(())
            },
            |_| Ok(()),
        );

        assert_eq!(result, Some(PathBuf::from("/fake/primary")));
        assert_eq!(attempts, vec![PathBuf::from("/fake/primary")]);
    }

    #[test]
    fn webview2_skips_candidates_that_fail_write_check() {
        let candidates: Vec<(&str, PathBuf)> = vec![
            ("primary", PathBuf::from("/fake/primary")),
            ("home", PathBuf::from("/fake/home")),
        ];

        let result = resolve_webview2_data_dir_with_write_check(
            &candidates,
            |_| Ok(()),
            |p| {
                if p == std::path::Path::new("/fake/primary") {
                    Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Access Denied",
                    ))
                } else {
                    Ok(())
                }
            },
        );

        assert_eq!(result, Some(PathBuf::from("/fake/home")));
    }
}
