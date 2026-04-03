// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use peekoo_agent_app::{
    AgentApplication, AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto,
    InstallProviderRequest, InstallProviderResponse, InstallationMethod, LastSessionDto,
    OauthCancelResponse, OauthStartResponse, OauthStatusRequest, OauthStatusResponse,
    PluginConfigFieldDto, PluginNotificationDto, PluginPanelDto, PluginSummaryDto,
    PomodoroCycleDto, PomodoroSettingsInput, PomodoroStatusDto, PrerequisitesCheck,
    ProviderAuthDto, ProviderConfig, ProviderConfigDto, ProviderInfo, ProviderRequest,
    RuntimeAuthenticationResult, RuntimeAuthenticationStatus, RuntimeInfo, RuntimeInspectionResult,
    RuntimeTerminalAuthLaunch, SetApiKeyRequest, SetProviderConfigRequest, SpriteInfo,
    StorePluginDto, TaskDto, TaskEventDto, TestConnectionResult,
};
use rusqlite::Connection;
use serde::Serialize;
use std::env;
use std::path::PathBuf;
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
const TASKS_CHANGED_EVENT: &str = "tasks-changed";
const SETTING_APP_LANGUAGE: &str = "app_language";
const AGENT_SETTINGS_CHANGED_EVENT: &str = "agent-settings-changed";
const SETTING_LOG_LEVEL: &str = "log_level";

mod tray_i18n;

rust_i18n::i18n!("locales", fallback = "en");

#[cfg(target_os = "macos")]
fn quote_posix_shell(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\"'\"'"))
}

#[cfg(target_os = "windows")]
fn quote_windows_cmd(arg: &str) -> String {
    format!("\"{}\"", arg.replace('"', "\"\""))
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn write_terminal_auth_script(
    extension: &str,
    command: &str,
    args: &[String],
    env_vars: &std::collections::HashMap<String, String>,
) -> Result<PathBuf, String> {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("terminal auth clock error: {e}"))?
        .as_millis();
    let path = std::env::temp_dir().join(format!("peekoo-runtime-auth-{unique}.{extension}"));

    #[cfg(target_os = "macos")]
    let content = {
        let mut lines = vec!["#!/bin/bash".to_string()];
        for (key, value) in env_vars {
            lines.push(format!("export {}={}", key, quote_posix_shell(value)));
        }
        let mut command_line = vec![quote_posix_shell(command)];
        command_line.extend(args.iter().map(|arg| quote_posix_shell(arg)));
        lines.push(command_line.join(" "));
        lines.push("exit".to_string());
        lines.join("\n")
    };

    #[cfg(target_os = "windows")]
    let content = {
        let mut lines = vec!["@echo off".to_string()];
        for (key, value) in env_vars {
            lines.push(format!("set \"{}={}\"", key, value));
        }
        let mut command_line = vec![quote_windows_cmd(command)];
        command_line.extend(args.iter().map(|arg| quote_windows_cmd(arg)));
        lines.push(command_line.join(" "));
        lines.join("\r\n")
    };

    fs::write(&path, content).map_err(|e| format!("terminal auth script write error: {e}"))?;

    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&path)
            .map_err(|e| format!("terminal auth script metadata error: {e}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions)
            .map_err(|e| format!("terminal auth script chmod error: {e}"))?;
    }

    Ok(path)
}

type TerminalLauncher = fn(&RuntimeTerminalAuthLaunch) -> Vec<String>;

#[cfg(target_os = "linux")]
fn launch_terminal_auth(launch: &RuntimeTerminalAuthLaunch) -> Result<(), String> {
    let candidates: [(&str, TerminalLauncher); 8] = [
        ("x-terminal-emulator", |launch| {
            let mut args = vec!["-e".to_string(), launch.command.clone()];
            args.extend(launch.args.clone());
            args
        }),
        ("gnome-terminal", |launch| {
            let mut args = vec![
                "--wait".to_string(),
                "--".to_string(),
                launch.command.clone(),
            ];
            args.extend(launch.args.clone());
            args
        }),
        ("konsole", |launch| {
            let mut args = vec!["-e".to_string(), launch.command.clone()];
            args.extend(launch.args.clone());
            args
        }),
        ("kitty", |launch| {
            let mut args = vec![launch.command.clone()];
            args.extend(launch.args.clone());
            args
        }),
        ("ghostty", |launch| {
            let mut args = vec!["-e".to_string(), launch.command.clone()];
            args.extend(launch.args.clone());
            args
        }),
        ("wezterm", |launch| {
            let mut args = vec![
                "start".to_string(),
                "--".to_string(),
                launch.command.clone(),
            ];
            args.extend(launch.args.clone());
            args
        }),
        ("alacritty", |launch| {
            let mut args = vec!["-e".to_string(), launch.command.clone()];
            args.extend(launch.args.clone());
            args
        }),
        ("xterm", |launch| {
            let mut args = vec!["-e".to_string(), launch.command.clone()];
            args.extend(launch.args.clone());
            args
        }),
    ];

    let mut last_error = None;
    for (terminal, build_args) in candidates {
        match Command::new(terminal)
            .args(build_args(launch))
            .envs(&launch.env)
            .spawn()
        {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                last_error = Some(format!("{terminal} launch error: {error}"));
                break;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        "No supported terminal emulator was found to launch runtime login.".to_string()
    }))
}

#[cfg(target_os = "macos")]
fn launch_terminal_auth(launch: &RuntimeTerminalAuthLaunch) -> Result<(), String> {
    let script = write_terminal_auth_script("command", &launch.command, &launch.args, &launch.env)?;
    Command::new("open")
        .arg("-a")
        .arg("Terminal")
        .arg(script)
        .spawn()
        .map_err(|e| format!("Terminal launch error: {e}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_terminal_auth(launch: &RuntimeTerminalAuthLaunch) -> Result<(), String> {
    let script = write_terminal_auth_script("cmd", &launch.command, &launch.args, &launch.env)?;
    Command::new("cmd")
        .args(["/C", "start", "", script.to_string_lossy().as_ref()])
        .spawn()
        .map_err(|e| format!("Terminal launch error: {e}"))?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayMenuAction {
    ToggleVisible,
    OpenSettings,
    OpenAbout,
    Quit,
}

fn apply_tray_menu_language(app: &AppHandle, language: &str) -> Result<(), String> {
    tray_i18n::set_tray_locale(language);
    let tray_menu = MenuBuilder::new(app)
        .text(TRAY_TOGGLE_MENU_ID, tray_i18n::tray_toggle())
        .text(TRAY_SETTINGS_MENU_ID, tray_i18n::tray_settings())
        .text(TRAY_ABOUT_MENU_ID, tray_i18n::tray_about())
        .separator()
        .text(TRAY_QUIT_MENU_ID, tray_i18n::tray_quit())
        .build()
        .map_err(|e| format!("Build tray menu error: {e}"))?;

    if let Some(tray) = app.tray_by_id(TRAY_ICON_ID) {
        tray.set_menu(Some(tray_menu))
            .map_err(|e| format!("Set tray menu error: {e}"))?;
        tray.set_tooltip(Some("Peekoo"))
            .map_err(|e| format!("Set tray tooltip error: {e}"))?;
    }

    Ok(())
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

fn resolve_default_log_level(
    rust_log_env: Option<String>,
    persisted_log_level: Option<String>,
    fallback_level: log::LevelFilter,
) -> log::LevelFilter {
    rust_log_env
        .as_deref()
        .and_then(parse_log_level)
        .or_else(|| persisted_log_level.as_deref().and_then(parse_log_level))
        .unwrap_or(fallback_level)
}

fn parse_log_level(value: &str) -> Option<log::LevelFilter> {
    value.parse::<log::LevelFilter>().ok()
}

fn read_persisted_log_level() -> Option<String> {
    let db_path = peekoo_paths::peekoo_settings_db_path().ok()?;
    let parent = db_path.parent()?;
    if !parent.exists() || !db_path.exists() {
        return None;
    }

    let conn = Connection::open(db_path).ok()?;
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [SETTING_LOG_LEVEL],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

struct AgentState {
    app: AgentApplication,
}

impl AgentState {
    fn new(app_handle: &AppHandle) -> Self {
        let bundled_opencode_path = resolve_bundled_opencode_path(app_handle);
        let bundled_acp_path = resolve_bundled_acp_path(app_handle);
        let bundled_node_bin_dir = resolve_bundled_node_bin_dir(app_handle);

        // If no bundled opencode, check previously-downloaded or system PATH.
        let opencode_path = bundled_opencode_path.or_else(resolve_opencode_fallback_path);

        Self {
            app: AgentApplication::new_with_bundled_binaries(
                opencode_path,
                bundled_acp_path,
                bundled_node_bin_dir,
            )
            .unwrap_or_else(|e| panic!("Failed to initialize agent application: {e}")),
        }
    }

    /// Whether opencode needs to be downloaded from the ACP registry.
    /// Returns `true` when no bundled, downloaded, or system opencode was found.
    fn needs_opencode_download(app_handle: &AppHandle) -> bool {
        resolve_bundled_opencode_path(app_handle).is_none()
            && resolve_opencode_fallback_path().is_none()
    }
}

fn resolve_bundled_opencode_path(app: &AppHandle) -> Option<PathBuf> {
    let resource_dir = app.path().resource_dir().ok()?;
    let opencode_dir = resource_dir.join("opencode");

    if cfg!(windows) {
        // Try the npm wrapper first, then fall back to the legacy direct binary.
        // Some Windows users may have either depending on which release they
        // installed — the bundling strategy switched from direct binary fetch
        // (opencode.exe) to npm-based wrapper (opencode.cmd).
        for file_name in &["opencode.cmd", "opencode.exe"] {
            let candidate = opencode_dir.join(file_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    } else {
        let candidate = opencode_dir.join("opencode");
        candidate.is_file().then_some(candidate)
    }
}

/// Check for a previously-installed opencode (via ACP registry) or one on the system PATH.
fn resolve_opencode_fallback_path() -> Option<PathBuf> {
    // Check ACP-registry install dir: ~/.peekoo/resources/agents/opencode/
    // The .installed marker written by `acp_registry_client::mark_installed`
    // contains the executable path; fall back to a glob search if missing.
    if let Ok(data_dir) = peekoo_paths::peekoo_global_data_dir() {
        let agent_dir = data_dir.join("resources").join("agents").join("opencode");
        let marker = agent_dir.join(".installed");
        if marker.is_file() {
            // Marker is JSON with an `executable_path` field.
            if let Ok(json) = std::fs::read_to_string(&marker) {
                if let Ok(info) = serde_json::from_str::<serde_json::Value>(&json) {
                    if let Some(exe) = info.get("executable_path").and_then(|v| v.as_str()) {
                        let path = PathBuf::from(exe);
                        if path.is_file() {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }

    // Check system PATH
    which::which("opencode").ok()
}

/// Spawn a background task to install opencode from the ACP registry.
///
/// Uses `install_registry_agent` so version resolution, download, extraction,
/// and DB seeding all go through the same path as user-initiated installs.
fn spawn_opencode_registry_install(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tracing::info!("OpenCode not found, installing from ACP registry...");

        let state = app_handle.state::<AgentState>();
        match state
            .app
            .install_registry_agent("opencode", InstallationMethod::Binary)
            .await
        {
            Ok(resp) => {
                tracing::info!("OpenCode installed from registry: {}", resp.message);
                let _ = app_handle.emit_to(MAIN_WINDOW_LABEL, AGENT_SETTINGS_CHANGED_EVENT, ());
            }
            Err(err) => {
                tracing::warn!("OpenCode registry install failed: {err}");
            }
        }
    });
}

fn resolve_bundled_acp_path(app: &AppHandle) -> Option<PathBuf> {
    let file_name = if cfg!(windows) {
        "peekoo-agent-acp.exe"
    } else {
        "peekoo-agent-acp"
    };

    app.path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join(file_name))
        .filter(|path| path.exists() && path.is_file())
}

fn resolve_bundled_node_bin_dir(app: &AppHandle) -> Option<PathBuf> {
    let bin_dir = if cfg!(windows) {
        // node.exe lives directly in the node/ directory on Windows
        "node"
    } else {
        "node/bin"
    };

    app.path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("opencode").join(bin_dir))
        .filter(|path| path.exists() && path.is_dir())
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

#[derive(Clone, Serialize)]
struct TaskChangeEvent {
    task_id: Option<String>,
}

fn emit_tasks_changed(app: &AppHandle, task_id: Option<&str>) {
    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        TASKS_CHANGED_EVENT,
        TaskChangeEvent {
            task_id: task_id.map(|id| id.to_string()),
        },
    );
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
    let message_len = message.chars().count();
    tracing::info!(message_len, "agent_prompt command received");
    let reply = state
        .app
        .prompt_streaming(&message, move |event| {
            let _ = window.emit("agent-event", event);
        })
        .await
        .map_err(|err| {
            tracing::error!(error = %err, message_len, "agent_prompt command failed");
            // Propagate structured auth_required errors so the frontend can
            // show a targeted login prompt instead of a raw error string.
            if let Some(runtime_id) = err.strip_prefix("AUTH_REQUIRED:") {
                return format!(r#"{{"code":"auth_required","runtimeId":"{}"}}"#, runtime_id);
            }
            err
        })?;
    tracing::info!(
        message_len,
        response_len = reply.chars().count(),
        "agent_prompt command completed"
    );
    Ok(AgentResponse { response: reply })
}

#[tauri::command]
async fn agent_settings_get(state: State<'_, AgentState>) -> Result<AgentSettingsDto, String> {
    state.app.get_settings()
}

#[tauri::command]
async fn agent_settings_update(
    patch: AgentSettingsPatchDto,
    app: AppHandle,
    state: State<'_, AgentState>,
) -> Result<AgentSettingsDto, String> {
    let settings = state.app.update_settings(patch)?;
    let _ = app.emit(AGENT_SETTINGS_CHANGED_EVENT, ());
    Ok(settings)
}

#[tauri::command]
async fn agent_settings_catalog(
    state: State<'_, AgentState>,
) -> Result<AgentSettingsCatalogDto, String> {
    state.app.settings_catalog().await
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

// ============================================================================
// ACP Runtime Management Commands
// ============================================================================

#[tauri::command]
async fn list_agent_providers(state: State<'_, AgentState>) -> Result<Vec<ProviderInfo>, String> {
    state.app.list_agent_providers()
}

#[tauri::command]
async fn install_agent_provider(
    req: InstallProviderRequest,
    state: State<'_, AgentState>,
) -> Result<InstallProviderResponse, String> {
    state.app.install_agent_provider(req)
}

#[tauri::command]
async fn uninstall_agent_provider(
    provider_id: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.uninstall_agent_provider(&provider_id)
}

#[tauri::command]
async fn set_default_provider(
    provider_id: String,
    app: AppHandle,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.set_default_agent_provider(&provider_id)?;
    let _ = app.emit(AGENT_SETTINGS_CHANGED_EVENT, ());
    Ok(())
}

#[tauri::command]
async fn get_provider_config(
    provider_id: String,
    state: State<'_, AgentState>,
) -> Result<ProviderConfig, String> {
    state.app.get_agent_provider_config(&provider_id)
}

#[tauri::command]
async fn update_provider_config(
    provider_id: String,
    config: ProviderConfig,
    app: AppHandle,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state
        .app
        .update_agent_provider_config(&provider_id, &config)?;
    let _ = app.emit(AGENT_SETTINGS_CHANGED_EVENT, ());
    Ok(())
}

#[tauri::command]
async fn test_provider_connection(
    provider_id: String,
    state: State<'_, AgentState>,
) -> Result<TestConnectionResult, String> {
    state.app.test_agent_provider_connection(&provider_id).await
}

#[tauri::command]
async fn check_installation_prerequisites(
    method: String,
    state: State<'_, AgentState>,
) -> Result<PrerequisitesCheck, String> {
    let method = match method.as_str() {
        "bundled" => InstallationMethod::Bundled,
        "npx" => InstallationMethod::Npx,
        "binary" => InstallationMethod::Binary,
        _ => InstallationMethod::Custom,
    };
    state.app.check_agent_provider_prerequisites(method)
}

#[tauri::command]
async fn add_custom_provider(
    name: String,
    description: Option<String>,
    command: String,
    args: Vec<String>,
    working_dir: Option<String>,
    state: State<'_, AgentState>,
) -> Result<ProviderInfo, String> {
    state.app.add_custom_agent_provider(
        &name,
        description.as_deref(),
        &command,
        &args,
        working_dir.as_deref(),
    )
}

#[tauri::command]
async fn remove_custom_provider(
    provider_id: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.remove_custom_agent_provider(&provider_id)
}

#[tauri::command]
async fn list_agent_runtimes(state: State<'_, AgentState>) -> Result<Vec<RuntimeInfo>, String> {
    state.app.list_agent_runtimes()
}

#[tauri::command]
async fn install_agent_runtime(
    req: InstallProviderRequest,
    state: State<'_, AgentState>,
) -> Result<InstallProviderResponse, String> {
    state.app.install_agent_runtime(req)
}

#[tauri::command]
async fn uninstall_agent_runtime(
    runtime_id: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.uninstall_agent_runtime(&runtime_id)
}

#[tauri::command]
async fn set_default_agent_runtime(
    runtime_id: String,
    app: AppHandle,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.set_default_agent_runtime(&runtime_id)?;
    let _ = app.emit(AGENT_SETTINGS_CHANGED_EVENT, ());
    Ok(())
}

#[tauri::command]
async fn inspect_runtime(
    runtime_id: String,
    state: State<'_, AgentState>,
) -> Result<RuntimeInspectionResult, String> {
    state.app.inspect_runtime(&runtime_id).await
}

#[tauri::command]
async fn authenticate_runtime(
    runtime_id: String,
    method_id: String,
    state: State<'_, AgentState>,
) -> Result<RuntimeAuthenticationResult, String> {
    match state
        .app
        .authenticate_runtime(&runtime_id, &method_id)
        .await?
    {
        peekoo_agent_app::agent_provider_commands::RuntimeAuthenticationAction::Authenticated {
            message,
        } => Ok(RuntimeAuthenticationResult {
            status: RuntimeAuthenticationStatus::Authenticated,
            message,
        }),
        peekoo_agent_app::agent_provider_commands::RuntimeAuthenticationAction::LaunchTerminal(
            launch,
        ) => {
            launch_terminal_auth(&launch)?;
            Ok(RuntimeAuthenticationResult {
                status: RuntimeAuthenticationStatus::TerminalLoginStarted,
                message: launch.message,
            })
        }
    }
}

#[tauri::command]
async fn refresh_runtime_capabilities(
    runtime_id: String,
    state: State<'_, AgentState>,
) -> Result<RuntimeInspectionResult, String> {
    state.app.refresh_runtime_capabilities(&runtime_id).await
}

// ============================================================================
// ACP Registry Commands
// ============================================================================

#[tauri::command]
async fn get_registry_agents(
    page: usize,
    page_size: usize,
    search_query: Option<String>,
    platform_only: bool,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    use peekoo_agent_app::{RegistryFilterOptions, RegistrySortBy};

    let filter = RegistryFilterOptions {
        search_query,
        platform_only,
        sort_by: RegistrySortBy::Featured,
        page: page.max(1),
        page_size: page_size.clamp(1, 100),
        method_filter: None,
    };

    let (agents, total_count) = state
        .app
        .fetch_registry_agents(&filter)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "agents": agents,
        "totalCount": total_count,
        "page": page,
        "pageSize": page_size,
        "hasMore": (page * page_size) < total_count,
    }))
}

#[tauri::command]
async fn search_registry_agents(
    query: String,
    state: State<'_, AgentState>,
) -> Result<Vec<serde_json::Value>, String> {
    let agents = state
        .app
        .search_registry_agents(&query)
        .await
        .map_err(|e| e.to_string())?;

    Ok(agents
        .into_iter()
        .map(|a| serde_json::to_value(a).unwrap())
        .collect())
}

#[tauri::command]
async fn install_registry_agent(
    registry_id: String,
    method: String,
    state: State<'_, AgentState>,
) -> Result<InstallProviderResponse, String> {
    use peekoo_agent_app::InstallationMethod;

    let install_method = match method.as_str() {
        "npx" => InstallationMethod::Npx,
        "binary" => InstallationMethod::Binary,
        _ => return Err(format!("Unsupported installation method: {}", method)),
    };

    state
        .app
        .install_registry_agent(&registry_id, install_method)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn refresh_registry_catalog(state: State<'_, AgentState>) -> Result<(), String> {
    state
        .app
        .refresh_registry()
        .await
        .map_err(|e| e.to_string())
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

#[tauri::command]
async fn system_open_log_dir(app: AppHandle) -> Result<(), String> {
    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("Get log dir error: {e}"))?;
    #[allow(deprecated)]
    app.shell()
        .open(log_dir.to_string_lossy().to_string(), None)
        .map(|_| ())
        .map_err(|e| format!("Open log dir error: {e}"))
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
    app: AppHandle,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.set_app_setting(&key, &value)?;
    if key == SETTING_APP_LANGUAGE {
        apply_tray_menu_language(&app, &value)?;
    }
    Ok(())
}

#[tauri::command]
async fn app_settings_list_sprites(
    state: State<'_, AgentState>,
) -> Result<Vec<SpriteInfo>, String> {
    Ok(state.app.list_sprites())
}

#[tauri::command]
async fn app_settings_get_language(state: State<'_, AgentState>) -> Result<String, String> {
    state.app.get_app_language()
}

#[tauri::command]
async fn app_settings_set_language(
    language: String,
    app: AppHandle,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.app.set_app_language(&language)?;
    apply_tray_menu_language(&app, &language)?;
    Ok(())
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
    _app: AppHandle,
) -> Result<TaskDto, String> {
    let assignee = assignee.as_deref().unwrap_or("user");
    let labels = labels.as_deref().unwrap_or(&[]);
    let task = state.app.create_task(
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
    )?;
    Ok(task)
}

#[tauri::command]
async fn create_task_from_text(
    text: String,
    state: State<'_, AgentState>,
    _app: AppHandle,
) -> Result<TaskDto, String> {
    let task = state.app.create_task_from_text(&text)?;
    Ok(task)
}

#[tauri::command]
async fn list_tasks(state: State<'_, AgentState>) -> Result<Vec<TaskDto>, String> {
    state.app.list_tasks()
}

#[tauri::command]
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
    recurrence_rule: Option<Option<String>>,
    recurrence_time_of_day: Option<Option<String>>,
    state: State<'_, AgentState>,
    _app: AppHandle,
) -> Result<TaskDto, String> {
    let task = state.app.update_task(
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
    )?;
    Ok(task)
}

#[tauri::command]
async fn delete_task(
    id: String,
    state: State<'_, AgentState>,
    _app: AppHandle,
) -> Result<(), String> {
    state.app.delete_task(&id)?;
    Ok(())
}

#[tauri::command]
async fn toggle_task(
    id: String,
    state: State<'_, AgentState>,
    _app: AppHandle,
) -> Result<TaskDto, String> {
    let task = state.app.toggle_task(&id)?;
    Ok(task)
}

#[tauri::command]
async fn get_task_activity(
    task_id: String,
    limit: Option<u32>,
    state: State<'_, AgentState>,
) -> Result<Vec<TaskEventDto>, String> {
    state.app.get_task_activity(&task_id, limit.unwrap_or(50))
}

#[tauri::command]
async fn task_list_events(
    limit: Option<i64>,
    state: State<'_, AgentState>,
) -> Result<Vec<TaskEventDto>, String> {
    state.app.list_task_events(limit.unwrap_or(50))
}

#[tauri::command]
async fn add_task_comment(
    task_id: String,
    text: String,
    author: String,
    state: State<'_, AgentState>,
    _app: AppHandle,
) -> Result<TaskEventDto, String> {
    let event = state.app.add_task_comment(&task_id, &text, &author)?;
    Ok(event)
}

#[tauri::command]
async fn delete_task_event(
    event_id: String,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<(), String> {
    state.app.delete_task_event(&event_id)?;
    emit_tasks_changed(&app, None);
    Ok(())
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
#[allow(clippy::too_many_arguments)]
async fn pomodoro_set_settings(
    work_minutes: u32,
    break_minutes: u32,
    long_break_minutes: u32,
    long_break_interval: u32,
    enable_memo: bool,
    auto_advance: bool,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroStatusDto, String> {
    let status = state.app.pomodoro_set_settings(PomodoroSettingsInput {
        work_minutes,
        break_minutes,
        long_break_minutes,
        long_break_interval,
        enable_memo,
        auto_advance,
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
async fn pomodoro_history_by_date_range(
    start_date: String,
    end_date: String,
    limit: usize,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<Vec<PomodoroCycleDto>, String> {
    let history = state
        .app
        .pomodoro_history_by_date_range(start_date, end_date, limit)?;
    flush_plugin_notifications(&app, &state)?;
    Ok(history)
}

#[tauri::command]
async fn pomodoro_save_memo(
    id: Option<String>,
    memo: String,
    state: State<'_, AgentState>,
    app: AppHandle,
) -> Result<PomodoroStatusDto, String> {
    let status = state.app.save_pomodoro_memo(id, memo)?;
    flush_plugin_notifications(&app, &state)?;
    Ok(status)
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
        .call_plugin_panel_tool(&plugin_key, &tool_name, &args_json)
        .map_err(|error| {
            tracing::error!(
                plugin_key = %plugin_key,
                tool_name = %tool_name,
                error = %error,
                "plugin_call_panel_tool failed"
            );
            error
        })?;
    flush_plugin_notifications(&app, &state)?;
    Ok(result)
}

#[tauri::command]
async fn plugin_query_data(
    plugin_key: String,
    provider_name: String,
    state: State<'_, AgentState>,
) -> Result<String, String> {
    state
        .app
        .query_plugin_data(&plugin_key, &provider_name)
        .map_err(|error| {
            tracing::error!(
                plugin_key = %plugin_key,
                provider_name = %provider_name,
                error = %error,
                "plugin_query_data failed"
            );
            error
        })
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

    let fallback_level = if cfg!(debug_assertions) {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Error
    };
    let default_level = resolve_default_log_level(
        env::var("RUST_LOG").ok(),
        read_persisted_log_level(),
        fallback_level,
    );

    let file_target = if cfg!(debug_assertions) {
        let log_dir = env::var("PEEKOO_PROJECT_ROOT")
            .map(|v| PathBuf::from(v.trim()))
            .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join("logs");

        // Ensure log directory exists before plugin initialization
        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            eprintln!(
                "Warning: Failed to create log directory at {:?}: {}",
                log_dir, e
            );
        }

        // Verify the directory exists and is accessible
        if !log_dir.exists() || !log_dir.is_dir() {
            eprintln!(
                "Warning: Log directory does not exist or is not accessible: {:?}",
                log_dir
            );
            // Fallback to LogDir which uses system temp/app data
            Target::new(TargetKind::LogDir { file_name: None })
        } else {
            Target::new(TargetKind::Folder {
                path: log_dir,
                file_name: None,
            })
        }
    } else {
        Target::new(TargetKind::LogDir { file_name: None })
    };

    tauri::Builder::default()
        .setup(|app| {
            let needs_opencode = AgentState::needs_opencode_download(app.handle());
            app.manage(AgentState::new(app.handle()));

            if needs_opencode {
                spawn_opencode_registry_install(app.handle().clone());
            }

            let initial_language = app
                .state::<AgentState>()
                .app
                .get_app_language()
                .unwrap_or_else(|_| "en".to_string());
            tray_i18n::set_tray_locale(&initial_language);

            let tray_menu = MenuBuilder::new(app)
                .text(TRAY_TOGGLE_MENU_ID, tray_i18n::tray_toggle())
                .text(TRAY_SETTINGS_MENU_ID, tray_i18n::tray_settings())
                .text(TRAY_ABOUT_MENU_ID, tray_i18n::tray_about())
                .separator()
                .text(TRAY_QUIT_MENU_ID, tray_i18n::tray_quit())
                .build()?;

            let mut tray_builder = tauri::tray::TrayIconBuilder::with_id(TRAY_ICON_ID)
                .menu(&tray_menu)
                .tooltip("Peekoo")
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
                // macOS template icons are intended for monochrome assets.
                // Our bundled app icon is a full-color asset, so forcing
                // template rendering makes the menu bar icon look distorted.
                tray_builder = tray_builder.icon_as_template(false);
            }

            let _ = tray_builder.build(app)?;

            let state = app.state::<AgentState>();
            let task_change_app = app.handle().clone();
            if let Err(err) =
                state
                    .app
                    .set_task_change_callback(std::sync::Arc::new(move |task_id| {
                        emit_tasks_changed(&task_change_app, task_id.as_deref());
                    }))
            {
                return Err(std::io::Error::other(err).into());
            }
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
            // Agent Provider Management
            list_agent_providers,
            install_agent_provider,
            uninstall_agent_provider,
            set_default_provider,
            get_provider_config,
            update_provider_config,
            test_provider_connection,
            check_installation_prerequisites,
            add_custom_provider,
            remove_custom_provider,
            list_agent_runtimes,
            install_agent_runtime,
            uninstall_agent_runtime,
            set_default_agent_runtime,
            inspect_runtime,
            authenticate_runtime,
            refresh_runtime_capabilities,
            // ACP Registry Commands
            get_registry_agents,
            search_registry_agents,
            install_registry_agent,
            refresh_registry_catalog,
            agent_oauth_start,
            agent_oauth_status,
            agent_oauth_cancel,
            system_open_url,
            system_open_log_dir,
            app_settings_get,
            app_settings_set,
            app_settings_list_sprites,
            app_settings_get_language,
            app_settings_set_language,
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
            pomodoro_history_by_date_range,
            pomodoro_save_memo,
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
    process_plugin_notifications(
        state.app.drain_plugin_notifications(),
        |notification| show_plugin_notification(app, notification),
        |notification| {
            if app.get_webview_window(MAIN_WINDOW_LABEL).is_some() {
                let _ = app.emit_to(MAIN_WINDOW_LABEL, "sprite:bubble", notification);
            }
            Ok(())
        },
    )?;

    flush_peek_badges(app, state)?;
    flush_mood_reactions(app, state)?;
    Ok(())
}

fn process_plugin_notifications<S, E>(
    notifications: Vec<PluginNotificationDto>,
    mut show: S,
    mut emit_sprite: E,
) -> Result<(), String>
where
    S: FnMut(&PluginNotificationDto) -> Result<(), String>,
    E: FnMut(&PluginNotificationDto) -> Result<(), String>,
{
    let mut first_error: Option<String> = None;

    for notification in notifications {
        let mut delivered_any = false;

        if let Err(err) = show(&notification) {
            tracing::warn!(
                source_plugin = notification.source_plugin,
                title = notification.title,
                "System notification delivery failed: {err}"
            );
        } else {
            delivered_any = true;
        }

        if let Err(err) = emit_sprite(&notification) {
            if !delivered_any && first_error.is_none() {
                first_error = Some(err);
            }
        } else {
            tracing::debug!(
                source_plugin = notification.source_plugin,
                title = notification.title,
                panel_label = notification.panel_label,
                "Emitted sprite bubble notification"
            );
            delivered_any = true;
        }

        if !delivered_any && first_error.is_none() {
            first_error = Some("Notification could not be delivered".to_string());
        }
    }

    match first_error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

fn flush_mood_reactions(app: &AppHandle, state: &AgentState) -> Result<(), String> {
    for reaction in state.app.drain_mood_reactions() {
        if app.get_webview_window(MAIN_WINDOW_LABEL).is_some() {
            let _ = app.emit_to(
                MAIN_WINDOW_LABEL,
                "pet:react",
                &PetReactionPayload {
                    trigger: reaction.trigger,
                    sticky: Some(reaction.sticky),
                },
            );
        }
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
        if app.get_webview_window(MAIN_WINDOW_LABEL).is_some() {
            let _ = app.emit_to(MAIN_WINDOW_LABEL, "sprite:peek-badges", &badges);
        }
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
    use peekoo_agent_app::PluginNotificationDto;

    use super::{
        MainWindowVisibilityAction, TrayMenuAction, next_main_window_visibility_action,
        process_plugin_notifications, resolve_default_log_level, tray_menu_action,
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

    fn sample_notification() -> PluginNotificationDto {
        PluginNotificationDto {
            source_plugin: "tasks".to_string(),
            title: "Task due".to_string(),
            body: "Starts now".to_string(),
            action_url: None,
            action_label: None,
            panel_label: None,
        }
    }

    #[test]
    fn still_emits_sprite_bubble_when_system_notification_fails() {
        let mut emitted = Vec::new();

        let result = process_plugin_notifications(
            vec![sample_notification()],
            |_| Err("notification backend unavailable".to_string()),
            |notification| {
                emitted.push(notification.title.clone());
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert_eq!(emitted, vec!["Task due".to_string()]);
    }

    #[test]
    fn returns_error_when_no_notification_surface_succeeds() {
        let result = process_plugin_notifications(
            vec![sample_notification()],
            |_| Err("notification backend unavailable".to_string()),
            |_| Err("sprite emit failed".to_string()),
        );

        assert!(result.is_err());
    }

    #[test]
    fn log_level_prefers_env_override() {
        let level = resolve_default_log_level(
            Some("trace".to_string()),
            Some("error".to_string()),
            log::LevelFilter::Info,
        );

        assert_eq!(level, log::LevelFilter::Trace);
    }

    #[test]
    fn log_level_uses_persisted_setting_when_env_missing() {
        let level =
            resolve_default_log_level(None, Some("debug".to_string()), log::LevelFilter::Error);

        assert_eq!(level, log::LevelFilter::Debug);
    }

    #[test]
    fn log_level_falls_back_when_values_invalid() {
        let level = resolve_default_log_level(
            Some("invalid".to_string()),
            Some("also-invalid".to_string()),
            log::LevelFilter::Error,
        );

        assert_eq!(level, log::LevelFilter::Error);
    }
}
