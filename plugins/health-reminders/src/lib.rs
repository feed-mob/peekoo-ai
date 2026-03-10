#![no_main]

use extism_pdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const WATER_KEY: &str = "water";
const EYE_REST_KEY: &str = "eye_rest";
const STANDUP_KEY: &str = "standup";
const POMODORO_ACTIVE_KEY: &str = "pomodoro_active";

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

#[derive(Serialize, Deserialize)]
struct ScheduleSetRequest {
    key: String,
    interval_secs: u64,
    repeat: bool,
}

#[derive(Serialize, Deserialize)]
struct ScheduleCancelRequest {
    key: String,
}

#[derive(Serialize, Deserialize)]
struct ScheduleGetRequest {
    key: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct ScheduleInfo {
    owner: String,
    key: String,
    interval_secs: u64,
    repeat: bool,
    time_remaining_secs: u64,
}

#[derive(Serialize, Deserialize)]
struct ScheduleGetResponse {
    schedule: Option<ScheduleInfo>,
}

#[derive(Serialize, Deserialize)]
struct ConfigGetRequest {
    key: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ConfigGetResponse {
    value: Value,
}

#[host_fn]
extern "ExtismHost" {
    fn peekoo_state_get(input: Json<StateGetRequest>) -> Json<StateGetResponse>;
    fn peekoo_state_set(input: Json<StateSetRequest>) -> Json<Value>;
    fn peekoo_log(input: Json<LogRequest>) -> Json<Value>;
    fn peekoo_emit_event(input: Json<EmitEventRequest>) -> Json<Value>;
    fn peekoo_notify(input: Json<NotifyRequest>) -> Json<Value>;
    fn peekoo_schedule_set(input: Json<ScheduleSetRequest>) -> Json<Value>;
    fn peekoo_schedule_cancel(input: Json<ScheduleCancelRequest>) -> Json<Value>;
    fn peekoo_schedule_get(input: Json<ScheduleGetRequest>) -> Json<ScheduleGetResponse>;
    fn peekoo_config_get(input: Json<ConfigGetRequest>) -> Json<ConfigGetResponse>;
}

#[derive(Clone, Serialize, Deserialize)]
struct ReminderConfig {
    water_interval_min: u32,
    eye_rest_interval_min: u32,
    standup_interval_min: u32,
    suppress_during_pomodoro: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct ReminderState {
    reminder_type: String,
    interval_min: u32,
    time_remaining_secs: u64,
    active: bool,
}

#[derive(Serialize, Deserialize)]
struct HealthStatus {
    config: ReminderConfig,
    pomodoro_active: bool,
    reminders: Vec<ReminderState>,
}

#[derive(Serialize, Deserialize)]
struct DismissInput {
    reminder_type: String,
}

#[plugin_fn]
pub fn plugin_init(_input: String) -> FnResult<String> {
    ensure_default_state();
    sync_schedules();
    log_info("Health reminders plugin initialized");
    Ok(r#"{"status":"ok"}"#.to_string())
}

#[plugin_fn]
pub fn on_event(input: String) -> FnResult<String> {
    let event: Value = serde_json::from_str(&input)?;
    let event_name = event["event"].as_str().unwrap_or("");

    match event_name {
        "schedule:fired" => {
            if let Some(key) = event["payload"]["key"].as_str() {
                handle_schedule_fired(key);
            }
        }
        "pomodoro:started" => {
            set_pomodoro_active(true);
            if load_config().suppress_during_pomodoro {
                cancel_all_schedules();
            }
        }
        "pomodoro:finished" | "pomodoro:paused" | "pomodoro:resumed" => {
            set_pomodoro_active(false);
            sync_schedules();
        }
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
    if let Some(value) = patch["suppress_during_pomodoro"].as_bool() {
        config.suppress_during_pomodoro = value;
    }

    save_config(&config);
    sync_schedules();
    Ok(serde_json::to_string(&load_status())?)
}

#[plugin_fn]
pub fn tool_health_dismiss(input: String) -> FnResult<String> {
    let args: DismissInput = serde_json::from_str(&input)?;
    reset_schedule(&args.reminder_type);
    Ok(serde_json::to_string(&load_status())?)
}

#[plugin_fn]
pub fn data_health_reminder_status(_input: String) -> FnResult<String> {
    Ok(serde_json::to_string(&load_status())?)
}

fn ensure_default_state() {
    if state_get(POMODORO_ACTIVE_KEY).is_none() {
        state_set(POMODORO_ACTIVE_KEY, json!(false));
    }
    save_config(&load_config());
}

fn handle_schedule_fired(key: &str) {
    let title = "Health Reminder";
    let body = match key {
        WATER_KEY => "Time to drink water.",
        EYE_REST_KEY => "Rest your eyes for a moment.",
        STANDUP_KEY => "Stand up and stretch.",
        _ => return,
    };

    notify(title, body);
    emit_event("health:reminder-due", json!({ "reminder_type": key }));
}

fn sync_schedules() {
    cancel_all_schedules();
    let config = load_config();
    if load_pomodoro_active() && config.suppress_during_pomodoro {
        return;
    }

    schedule_set(WATER_KEY, u64::from(config.water_interval_min) * 60);
    schedule_set(EYE_REST_KEY, u64::from(config.eye_rest_interval_min) * 60);
    schedule_set(STANDUP_KEY, u64::from(config.standup_interval_min) * 60);
}

fn cancel_all_schedules() {
    for key in [WATER_KEY, EYE_REST_KEY, STANDUP_KEY] {
        schedule_cancel(key);
    }
}

fn reset_schedule(reminder_type: &str) {
    schedule_cancel(reminder_type);
    let config = load_config();
    let interval_secs = match reminder_type {
        WATER_KEY => u64::from(config.water_interval_min) * 60,
        EYE_REST_KEY => u64::from(config.eye_rest_interval_min) * 60,
        STANDUP_KEY => u64::from(config.standup_interval_min) * 60,
        _ => return,
    };
    schedule_set(reminder_type, interval_secs);
}

fn load_status() -> HealthStatus {
    let config = load_config();
    let pomodoro_active = load_pomodoro_active();

    HealthStatus {
        config: config.clone(),
        pomodoro_active,
        reminders: vec![
            load_reminder_state(WATER_KEY, config.water_interval_min),
            load_reminder_state(EYE_REST_KEY, config.eye_rest_interval_min),
            load_reminder_state(STANDUP_KEY, config.standup_interval_min),
        ],
    }
}

fn load_reminder_state(reminder_type: &str, interval_min: u32) -> ReminderState {
    let schedule = schedule_get(reminder_type);
    ReminderState {
        reminder_type: reminder_type.to_string(),
        interval_min,
        time_remaining_secs: schedule
            .as_ref()
            .map(|value| value.time_remaining_secs)
            .unwrap_or(0),
        active: schedule.is_some(),
    }
}

fn load_config() -> ReminderConfig {
    let config = config_get();
    ReminderConfig {
        water_interval_min: config["water_interval_min"].as_u64().unwrap_or(45) as u32,
        eye_rest_interval_min: config["eye_rest_interval_min"].as_u64().unwrap_or(20) as u32,
        standup_interval_min: config["standup_interval_min"].as_u64().unwrap_or(60) as u32,
        suppress_during_pomodoro: config["suppress_during_pomodoro"].as_bool().unwrap_or(true),
    }
}

fn save_config(config: &ReminderConfig) {
    state_set("water_interval_min", json!(config.water_interval_min));
    state_set("eye_rest_interval_min", json!(config.eye_rest_interval_min));
    state_set("standup_interval_min", json!(config.standup_interval_min));
    state_set(
        "suppress_during_pomodoro",
        json!(config.suppress_during_pomodoro),
    );
}

fn load_pomodoro_active() -> bool {
    state_get(POMODORO_ACTIVE_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn set_pomodoro_active(active: bool) {
    state_set(POMODORO_ACTIVE_KEY, json!(active));
}

fn config_get() -> Value {
    unsafe { peekoo_config_get(Json(ConfigGetRequest { key: None })) }
        .ok()
        .map(|response| response.0.value)
        .unwrap_or_else(|| json!({}))
}

fn schedule_get(key: &str) -> Option<ScheduleInfo> {
    unsafe {
        peekoo_schedule_get(Json(ScheduleGetRequest {
            key: key.to_string(),
        }))
    }
    .ok()
    .and_then(|response| response.0.schedule)
}

fn schedule_set(key: &str, interval_secs: u64) {
    let _ = unsafe {
        peekoo_schedule_set(Json(ScheduleSetRequest {
            key: key.to_string(),
            interval_secs,
            repeat: true,
        }))
    };
}

fn schedule_cancel(key: &str) {
    let _ = unsafe {
        peekoo_schedule_cancel(Json(ScheduleCancelRequest {
            key: key.to_string(),
        }))
    };
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
