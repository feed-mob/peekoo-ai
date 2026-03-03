// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use peekoo_agent_app::{
    AgentApplication, AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto,
    OauthCancelResponse, OauthStartResponse, OauthStatusRequest, OauthStatusResponse,
    ProviderAuthDto, ProviderConfigDto, ProviderRequest, SetApiKeyRequest,
    SetProviderConfigRequest,
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
        "mood": "happy",
        "message": "Welcome to Peekoo! Your AI desktop sprite is ready to help you!",
        "animation": "happy"
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
async fn create_task(title: String, priority: String) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "id": "task-123",
        "title": title,
        "priority": priority,
        "status": "todo"
    }))
}

// ============================================================================
// App Entry
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            create_task
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
