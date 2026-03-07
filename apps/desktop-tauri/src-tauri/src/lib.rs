// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use peekoo_agent_app::{
    AgentApplication, AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto,
    OauthCancelResponse, OauthStartResponse, OauthStatusRequest, OauthStatusResponse,
    PluginPanelDto, PluginSummaryDto, PomodoroSessionDto, ProviderAuthDto, ProviderConfigDto,
    ProviderRequest, SetApiKeyRequest, SetProviderConfigRequest, TaskDto,
};
use serde::Serialize;
use tauri::{Emitter, State, Window};
// ============================================================================
// Agent State — lazily initialized on first prompt
// ============================================================================

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

// ============================================================================
// Tauri Commands
// ============================================================================

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
) -> Result<PomodoroSessionDto, String> {
    state.app.start_pomodoro(minutes)
}

#[tauri::command]
async fn pomodoro_pause(
    session_id: String,
    state: State<'_, AgentState>,
) -> Result<PomodoroSessionDto, String> {
    state.app.pause_pomodoro(&session_id)
}

#[tauri::command]
async fn pomodoro_resume(
    session_id: String,
    state: State<'_, AgentState>,
) -> Result<PomodoroSessionDto, String> {
    state.app.resume_pomodoro(&session_id)
}

#[tauri::command]
async fn pomodoro_finish(
    session_id: String,
    state: State<'_, AgentState>,
) -> Result<PomodoroSessionDto, String> {
    state.app.finish_pomodoro(&session_id)
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
) -> Result<String, String> {
    state.app.call_plugin_tool(&tool_name, &args_json)
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
) -> Result<(), String> {
    state.app.dispatch_plugin_event(&event_name, &payload_json)
}

// ============================================================================
// App Entry
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // On Windows, WebView2 defaults to writing its data next to the executable,
    // which is typically inside Program Files and not writable. Set an explicit
    // user-writable path before Tauri initialises the webview.
    #[cfg(target_os = "windows")]
    {
        if std::env::var("WEBVIEW2_USER_DATA_FOLDER").is_err() {
            if let Some(mut data_dir) = dirs::data_local_dir() {
                data_dir.push("com.peekoo.desktop");
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

    let agent_state = AgentState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(agent_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            get_sprite_state,
            agent_prompt,
            agent_set_model,
            agent_get_model,
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
            plugin_dispatch_event
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
