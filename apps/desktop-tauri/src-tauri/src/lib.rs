// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
async fn greet(name: String) -> Result<String, ()> {
    Ok(format!("Hello, {}! This is Peekoo Desktop (Tauri Version)", name))
}

#[tauri::command]
async fn get_sprite_state() -> Result<serde_json::Value, ()> {
    // TODO: Integrate with core-domain
    Ok(serde_json::json!({
        "mood": "happy",
        "message": "Welcome to Peekoo! Your AI desktop sprite is ready to help you!",
        "animation": "happy"
    }))
}

#[tauri::command]
async fn send_message(message: String) -> Result<serde_json::Value, ()> {
    // TODO: Integrate with core-app for agent processing
    Ok(serde_json::json!({
        "response": format!("You said: {}", message),
        "pet_mood": "thinking"
    }))
}

#[tauri::command]
async fn create_task(title: String, priority: String) -> Result<serde_json::Value, ()> {
    // TODO: Integrate with core-domain
    Ok(serde_json::json!({
        "id": "task-123",
        "title": title,
        "priority": priority,
        "status": "todo"
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_sprite_state,
            send_message,
            create_task
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
