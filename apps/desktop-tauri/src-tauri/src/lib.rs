// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use peekoo_agent_app::{
    AgentApplication, AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto,
    LastSessionDto, OauthCancelResponse, OauthStartResponse, OauthStatusRequest,
    OauthStatusResponse, PluginConfigFieldDto, PluginNotificationDto, PluginPanelDto,
    PluginSummaryDto, PomodoroSessionDto, ProviderAuthDto, ProviderConfigDto, ProviderRequest,
    SetApiKeyRequest, SetProviderConfigRequest, StorePluginDto, TaskDto,
};
use serde::Serialize;
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, State, Window,
    image::Image,
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
};
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_notification::NotificationExt;
// ============================================================================
// Agent State — lazily initialized on first prompt
// ============================================================================

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ICON_ID: &str = "main-tray";
const TRAY_TOGGLE_MENU_ID: &str = "toggle_visible";
const TRAY_QUIT_MENU_ID: &str = "quit";
const TRAY_TOOLTIP: &str = "Peekoo";

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
    match menu_id {
        TRAY_TOGGLE_MENU_ID => toggle_main_window_visibility(app),
        TRAY_QUIT_MENU_ID => app.exit(0),
        _ => {}
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

/// Resize the sprite window from Rust, bypassing the `resizable: false` JS restriction.
/// The window is intentionally non-resizable by the user but we need programmatic control.
/// `delta_top` shifts the window vertically in logical pixels (positive = move up, negative = move down).
#[tauri::command]
async fn resize_sprite_window(
    width: f64,
    height: f64,
    delta_top: f64,
    window: Window,
) -> Result<(), String> {
    if delta_top.abs() > 0.5 {
        let pos = window
            .outer_position()
            .map_err(|e| format!("get position error: {e}"))?;
        let scale = window
            .scale_factor()
            .map_err(|e| format!("scale error: {e}"))?;
        let logical_y = pos.y as f64 / scale - delta_top;
        let physical_y = (logical_y * scale).round() as i32;
        window
            .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: pos.x,
                y: physical_y,
            }))
            .map_err(|e| format!("set position error: {e}"))?;
    }
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| format!("resize error: {e}"))
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
async fn create_task(
    title: String,
    priority: String,
    state: State<'_, AgentState>,
) -> Result<TaskDto, String> {
    state.app.create_task(&title, &priority)
}

#[tauri::command]
async fn pomodoro_start(
    minutes: u32,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroSessionDto, String> {
    let session = state.app.start_pomodoro(minutes)?;
    state.app.dispatch_plugin_event("pomodoro:started", "{}")?;
    flush_plugin_notifications(&app, &state)?;
    Ok(session)
}

#[tauri::command]
async fn pomodoro_pause(
    session_id: String,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroSessionDto, String> {
    let session = state.app.pause_pomodoro(&session_id)?;
    state.app.dispatch_plugin_event("pomodoro:paused", "{}")?;
    flush_plugin_notifications(&app, &state)?;
    Ok(session)
}

#[tauri::command]
async fn pomodoro_resume(
    session_id: String,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroSessionDto, String> {
    let session = state.app.resume_pomodoro(&session_id)?;
    state.app.dispatch_plugin_event("pomodoro:resumed", "{}")?;
    flush_plugin_notifications(&app, &state)?;
    Ok(session)
}

#[tauri::command]
async fn pomodoro_finish(
    session_id: String,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroSessionDto, String> {
    let session = state.app.finish_pomodoro(&session_id)?;
    state.app.dispatch_plugin_event("pomodoro:finished", "{}")?;
    flush_plugin_notifications(&app, &state)?;
    Ok(session)
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
// App Entry
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "windows")]
    {
        if std::env::var("WEBVIEW2_USER_DATA_FOLDER").is_err() {
            if let Some(mut data_dir) = dirs::data_local_dir() {
                if cfg!(debug_assertions) {
                    data_dir.push("com.peekoo.desktop.dev");
                } else {
                    data_dir.push("com.peekoo.desktop");
                }
                data_dir.push("WebView2");
                if let Err(e) = std::fs::create_dir_all(&data_dir) {
                    eprintln!("warning: failed to create WebView2 data dir: {e}");
                }
                // SAFETY: Called at the start of `run()` before `tauri::Builder`
                // is constructed, so no other threads are running yet.
                unsafe { std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", data_dir) };
            }
        }
    }

    let default_level = env::var("RUST_LOG")
        .ok()
        .and_then(|v| v.parse::<log::LevelFilter>().ok())
        .unwrap_or(log::LevelFilter::Error);

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
            create_task,
            pomodoro_start,
            pomodoro_pause,
            pomodoro_resume,
            pomodoro_finish,
            plugins_list,
            plugin_panels_list,
            plugin_call_tool,
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
            plugin_store_uninstall
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
    Ok(())
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
    use super::{MainWindowVisibilityAction, next_main_window_visibility_action};

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
}
