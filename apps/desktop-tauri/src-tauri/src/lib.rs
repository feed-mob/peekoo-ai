// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use peekoo_agent::config::AgentServiceConfig;
use peekoo_agent::service::AgentService;
use serde::Serialize;
use tauri::State;

// ============================================================================
// Agent State — lazily initialized on first prompt
// ============================================================================

struct AgentState {
    agent: Mutex<Option<AgentService>>,
}

impl AgentState {
    fn new() -> Self {
        Self {
            agent: Mutex::new(None),
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
    state: State<'_, AgentState>,
) -> Result<AgentResponse, String> {
    // Take the agent out of the mutex briefly to avoid holding the lock
    // across the await point.
    let mut agent = {
        let mut guard = state.agent.lock().map_err(|e| format!("Lock error: {e}"))?;

        // Lazy init on first call.
        if guard.is_none() {
            let config = AgentServiceConfig {
                system_prompt: Some(
                    "You are Peekoo, a friendly and helpful AI desktop pet. \
                     You live on the user's desktop and assist them with coding, \
                     writing, research, and everyday tasks. Be concise, warm, \
                     and proactive. Use emoji sparingly to add personality. \
                     When helping with code, be precise and show working examples."
                        .into(),
                ),
                ..Default::default()
            };

            let reactor = asupersync::runtime::reactor::create_reactor()
                .map_err(|e| format!("Reactor error: {e}"))?;
            let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
                .with_reactor(reactor)
                .build()
                .map_err(|e| format!("Runtime error: {e}"))?;

            let service = runtime.block_on(AgentService::new(config))
                .map_err(|e| format!("Agent init error: {e}"))?;
            *guard = Some(service);
        }

        guard.take().unwrap()
    };

    // Run the prompt in pi's runtime.
    let reactor = asupersync::runtime::reactor::create_reactor()
        .map_err(|e| format!("Reactor error: {e}"))?;
    let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
        .with_reactor(reactor)
        .build()
        .map_err(|e| format!("Runtime error: {e}"))?;

    let result = runtime.block_on(agent.prompt(&message, |_event| {}));

    // Put the agent back.
    {
        let mut guard = state.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
        *guard = Some(agent);
    }

    match result {
        Ok(reply) => Ok(AgentResponse { response: reply }),
        Err(e) => Err(format!("Agent error: {e}")),
    }
}

#[tauri::command]
async fn agent_set_model(
    provider: String,
    model: String,
    state: State<'_, AgentState>,
) -> Result<ModelInfo, String> {
    let mut agent = {
        let mut guard = state.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
        guard
            .take()
            .ok_or("Agent not initialized. Send a message first.")?
    };

    let reactor = asupersync::runtime::reactor::create_reactor()
        .map_err(|e| format!("Reactor error: {e}"))?;
    let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
        .with_reactor(reactor)
        .build()
        .map_err(|e| format!("Runtime error: {e}"))?;

    let result = runtime.block_on(agent.set_model(&provider, &model));

    // Put the agent back.
    {
        let mut guard = state.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
        *guard = Some(agent);
    }

    result
        .map(|_| ModelInfo {
            provider,
            model,
        })
        .map_err(|e| format!("Set model error: {e}"))
}

#[tauri::command]
async fn agent_get_model(state: State<'_, AgentState>) -> Result<ModelInfo, String> {
    let guard = state.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
    let agent = guard
        .as_ref()
        .ok_or("Agent not initialized. Send a message first.")?;

    let (provider, model) = agent.model();
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
            create_task
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
