#![no_main]

use extism_pdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
struct StateGetRequest {
    key: String,
}

#[derive(Serialize, Deserialize)]
struct StateGetResponse {
    value: Value,
}

#[derive(Serialize, Deserialize)]
struct StateSetRequest {
    key: String,
    value: Value,
}

#[derive(Serialize, Deserialize)]
struct LogRequest {
    level: String,
    message: String,
}

#[derive(Serialize, Deserialize)]
struct EmitEventRequest {
    event: String,
    payload: Value,
}

#[derive(Serialize, Deserialize)]
struct NotifyRequest {
    title: String,
    body: String,
}

#[host_fn]
extern "ExtismHost" {
    fn peekoo_state_get(input: Json<StateGetRequest>) -> Json<StateGetResponse>;
    fn peekoo_state_set(input: Json<StateSetRequest>) -> Json<Value>;
    fn peekoo_log(input: Json<LogRequest>) -> Json<Value>;
    fn peekoo_emit_event(input: Json<EmitEventRequest>) -> Json<Value>;
    fn peekoo_notify(input: Json<NotifyRequest>) -> Json<Value>;
}

#[derive(Clone, Serialize, Deserialize)]
struct ReminderConfig {
    water_interval_min: u32,
    eye_rest_interval_min: u32,
    standup_interval_min: u32,
}

impl Default for ReminderConfig {
    fn default() -> Self {
        Self {
            water_interval_min: 45,
            eye_rest_interval_min: 20,
            standup_interval_min: 60,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct ReminderState {
    reminder_type: String,
    interval_min: u32,
    minutes_since_last: u32,
    is_due: bool,
}

#[derive(Serialize, Deserialize)]
struct HealthStatus {
    config: ReminderConfig,
    reminders: Vec<ReminderState>,
}

#[derive(Serialize, Deserialize)]
struct DismissInput {
    reminder_type: String,
}

#[plugin_fn]
pub fn plugin_init(_input: String) -> FnResult<String> {
    ensure_default_state();
    log_info("Health reminders plugin initialized");
    Ok(r#"{"status":"ok"}"#.to_string())
}

#[plugin_fn]
pub fn on_event(input: String) -> FnResult<String> {
    let event: Value = serde_json::from_str(&input)?;
    let event_name = event["event"].as_str().unwrap_or("");

    match event_name {
        "timer:tick" => tick_all(),
        "pomodoro:finished" => tick_after_pomodoro(),
        _ => {}
    }

    Ok(r#"{"ok":true}"#.to_string())
}

#[plugin_fn]
pub fn tool_health_get_status(_input: String) -> FnResult<String> {
    Ok(serde_json::to_string(&load_status())?)
}

#[plugin_fn]
pub fn tool_health_configure(input: String) -> FnResult<String> {
    let patch: Value = serde_json::from_str(&input)?;
    let mut config = load_config();

    if let Some(value) = patch["water_interval_min"].as_u64() {
        config.water_interval_min = value as u32;
    }
    if let Some(value) = patch["eye_rest_interval_min"].as_u64() {
        config.eye_rest_interval_min = value as u32;
    }
    if let Some(value) = patch["standup_interval_min"].as_u64() {
        config.standup_interval_min = value as u32;
    }

    save_config(&config);
    Ok(serde_json::to_string(&load_status())?)
}

#[plugin_fn]
pub fn tool_health_dismiss(input: String) -> FnResult<String> {
    let args: DismissInput = serde_json::from_str(&input)?;
    reset_reminder(&args.reminder_type);
    Ok(serde_json::to_string(&load_status())?)
}

#[plugin_fn]
pub fn data_health_reminder_status(_input: String) -> FnResult<String> {
    Ok(serde_json::to_string(&load_status())?)
}

fn ensure_default_state() {
    if state_get("config").is_none() {
        save_config(&ReminderConfig::default());
    }
    for reminder in ["water", "eye_rest", "standup"] {
        if state_get(reminder).is_none() {
            state_set(
                reminder,
                json!({ "minutes_since_last": 0, "is_due": false }),
            );
        }
    }
}

fn tick_all() {
    let config = load_config();
    tick_reminder("water", config.water_interval_min, "Time to drink water.");
    tick_reminder(
        "eye_rest",
        config.eye_rest_interval_min,
        "Rest your eyes for a moment.",
    );
    tick_reminder(
        "standup",
        config.standup_interval_min,
        "Stand up and stretch.",
    );
}

fn tick_after_pomodoro() {
    tick_reminder(
        "water",
        load_config().water_interval_min,
        "Pomodoro done. Drink some water.",
    );
}

fn tick_reminder(reminder_type: &str, interval_min: u32, message: &str) {
    let mut state = state_get(reminder_type).unwrap_or_else(|| {
        json!({
            "minutes_since_last": 0,
            "is_due": false,
        })
    });

    let next_value = state["minutes_since_last"].as_u64().unwrap_or(0) + 1;
    state["minutes_since_last"] = json!(next_value);

    if next_value >= interval_min as u64 {
        state["is_due"] = json!(true);
        notify("Health Reminder", message);
        emit_event(
            "health:reminder-due",
            json!({ "reminder_type": reminder_type }),
        );
    }

    state_set(reminder_type, state);
}

fn reset_reminder(reminder_type: &str) {
    state_set(
        reminder_type,
        json!({
            "minutes_since_last": 0,
            "is_due": false,
        }),
    );
}

fn load_status() -> HealthStatus {
    let config = load_config();
    HealthStatus {
        reminders: vec![
            load_reminder_state("water", config.water_interval_min),
            load_reminder_state("eye_rest", config.eye_rest_interval_min),
            load_reminder_state("standup", config.standup_interval_min),
        ],
        config,
    }
}

fn load_reminder_state(reminder_type: &str, interval_min: u32) -> ReminderState {
    let value = state_get(reminder_type).unwrap_or_else(|| {
        json!({
            "minutes_since_last": 0,
            "is_due": false,
        })
    });

    ReminderState {
        reminder_type: reminder_type.to_string(),
        interval_min,
        minutes_since_last: value["minutes_since_last"].as_u64().unwrap_or(0) as u32,
        is_due: value["is_due"].as_bool().unwrap_or(false),
    }
}

fn load_config() -> ReminderConfig {
    state_get("config")
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

fn save_config(config: &ReminderConfig) {
    state_set(
        "config",
        serde_json::to_value(config).unwrap_or_else(|_| json!({})),
    );
}

fn state_get(key: &str) -> Option<Value> {
    let response = unsafe {
        peekoo_state_get(Json(StateGetRequest {
            key: key.to_string(),
        }))
    }
    .ok()?;
    if response.0.value.is_null() {
        None
    } else {
        Some(response.0.value)
    }
}

fn state_set(key: &str, value: Value) {
    let _ = unsafe {
        peekoo_state_set(Json(StateSetRequest {
            key: key.to_string(),
            value,
        }))
    };
}

fn log_info(message: &str) {
    let _ = unsafe {
        peekoo_log(Json(LogRequest {
            level: "info".to_string(),
            message: message.to_string(),
        }))
    };
}

fn emit_event(event: &str, payload: Value) {
    let _ = unsafe {
        peekoo_emit_event(Json(EmitEventRequest {
            event: event.to_string(),
            payload,
        }))
    };
}

fn notify(title: &str, body: &str) {
    let _ = unsafe {
        peekoo_notify(Json(NotifyRequest {
            title: title.to_string(),
            body: body.to_string(),
        }))
    };
}
