#![no_main]

use extism_pdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const WATER_KEY: &str = "water";
const EYE_REST_KEY: &str = "eye_rest";
const STANDUP_KEY: &str = "standup";

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
    #[serde(skip_serializing_if = "Option::is_none")]
    delay_secs: Option<u64>,
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

#[derive(Serialize, Deserialize)]
struct PeekBadgeItem {
    label: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    countdown_secs: Option<u64>,
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
    fn peekoo_set_peek_badge(input: String) -> Json<Value>;
}

#[derive(Clone, Serialize, Deserialize)]
struct ReminderConfig {
    water_interval_min: u32,
    eye_rest_interval_min: u32,
    standup_interval_min: u32,
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
    push_peek_badges();
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
        "system:wake" => {
            sync_schedules();
        }
        _ => {}
    }

    push_peek_badges();
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
        config.water_interval_min = (value as u32).clamp(5, 180);
    }
    if let Some(value) = patch["eye_rest_interval_min"].as_u64() {
        config.eye_rest_interval_min = (value as u32).clamp(5, 120);
    }
    if let Some(value) = patch["standup_interval_min"].as_u64() {
        config.standup_interval_min = (value as u32).clamp(10, 180);
    }
    save_config(&config);
    sync_schedules();
    push_peek_badges();
    Ok(serde_json::to_string(&load_status())?)
}

#[plugin_fn]
pub fn tool_health_dismiss(input: String) -> FnResult<String> {
    let args: DismissInput = serde_json::from_str(&input)?;
    reset_schedule(&args.reminder_type);
    push_peek_badges();
    Ok(serde_json::to_string(&load_status())?)
}

#[plugin_fn]
pub fn data_health_reminder_status(_input: String) -> FnResult<String> {
    Ok(serde_json::to_string(&load_status())?)
}

fn ensure_default_state() {
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

    // The scheduler auto-repeats, so persist the new fire-at timestamp
    // so that the countdown survives app restarts.
    if let Some(schedule) = schedule_get(key) {
        save_timer_started_at(key, schedule.interval_secs, None);
    }

    notify(title, body);
    emit_event("health:reminder-due", json!({ "reminder_type": key }));
}

fn sync_schedules() {
    cancel_all_schedules();
    let config = load_config();

    let reminders = [
        (WATER_KEY, u64::from(config.water_interval_min) * 60),
        (EYE_REST_KEY, u64::from(config.eye_rest_interval_min) * 60),
        (STANDUP_KEY, u64::from(config.standup_interval_min) * 60),
    ];

    let now = current_epoch_secs();
    for (key, interval_secs) in reminders {
        // Health reminders skip missed timers rather than firing immediately
        // on restart -- the user doesn't need a stale "drink water" alert.
        let delay = compute_remaining_delay(key, interval_secs, now, false);
        schedule_set_with_delay(key, interval_secs, delay);
    }
}

/// Compute the initial delay for a timer based on persisted state.
///
/// Returns `None` (use full interval) when there is no stored timestamp or
/// when the stored interval differs from the current config (interval was
/// changed while the app was closed).  Returns `Some(remaining)` when a
/// valid persisted fire-at epoch exists.
///
/// When the timer is overdue (fire_at <= now) and `fire_if_overdue` is
/// false, the missed reminder is skipped and the delay is set to the
/// remaining time in the *next* cycle.  For example, if a 45-min timer was
/// overdue by 2 min, the delay is 43 min -- not the full 45 min and not 0.
fn compute_remaining_delay(
    key: &str,
    interval_secs: u64,
    now_epoch: u64,
    fire_if_overdue: bool,
) -> Option<u64> {
    let (fire_at, stored_interval) = load_timer_fire_at(key)?;

    // If the configured interval changed, ignore the stored timestamp and
    // start fresh with the new interval.
    if stored_interval != interval_secs {
        return None;
    }

    if fire_at <= now_epoch {
        if fire_if_overdue {
            Some(0)
        } else {
            // Skip the missed reminder and compute how far into the next
            // cycle we are so the delay accounts for elapsed time.
            let overdue = now_epoch - fire_at;
            let into_next_cycle = overdue % interval_secs;
            let remaining = interval_secs - into_next_cycle;
            Some(remaining)
        }
    } else {
        Some(fire_at - now_epoch)
    }
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

    HealthStatus {
        config: config.clone(),
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
    }
}

fn save_config(config: &ReminderConfig) {
    state_set("water_interval_min", json!(config.water_interval_min));
    state_set("eye_rest_interval_min", json!(config.eye_rest_interval_min));
    state_set("standup_interval_min", json!(config.standup_interval_min));
}

/// Persist the wall-clock epoch when the timer for `key` will next fire.
///
/// We store `fire_at = now + effective_delay` so that on restart we can
/// compute `remaining = fire_at - now`.
fn save_timer_started_at(key: &str, interval_secs: u64, delay_secs: Option<u64>) {
    let effective_delay = delay_secs.unwrap_or(interval_secs);
    let fire_at = current_epoch_secs() + effective_delay;
    state_set(&format!("timer_fire_at:{key}"), json!(fire_at));
    state_set(&format!("timer_interval:{key}"), json!(interval_secs));
}

/// Load the persisted next-fire epoch for `key`.
/// Returns `(fire_at_epoch, interval_secs)` if both values exist.
fn load_timer_fire_at(key: &str) -> Option<(u64, u64)> {
    let fire_at = state_get(&format!("timer_fire_at:{key}"))?.as_u64()?;
    let interval = state_get(&format!("timer_interval:{key}"))?.as_u64()?;
    Some((fire_at, interval))
}

fn current_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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
    schedule_set_with_delay(key, interval_secs, None);
}

fn schedule_set_with_delay(key: &str, interval_secs: u64, delay_secs: Option<u64>) {
    let _ = unsafe {
        peekoo_schedule_set(Json(ScheduleSetRequest {
            key: key.to_string(),
            interval_secs,
            repeat: true,
            delay_secs,
        }))
    };
    save_timer_started_at(key, interval_secs, delay_secs);
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

fn push_peek_badges() {
    let status = load_status();
    let icon_for = |reminder_type: &str| -> &str {
        match reminder_type {
            "water" => "droplet",
            "eye_rest" => "eye",
            "standup" => "person-standing",
            _ => "activity",
        }
    };

    let items: Vec<PeekBadgeItem> = status
        .reminders
        .iter()
        .filter(|reminder| reminder.active)
        .map(|reminder| PeekBadgeItem {
            label: reminder
                .reminder_type
                .replace('_', " ")
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
            value: format_countdown(reminder.time_remaining_secs),
            icon: Some(icon_for(&reminder.reminder_type).to_string()),
            countdown_secs: Some(reminder.time_remaining_secs),
        })
        .collect();

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string());
    let _ = unsafe { peekoo_set_peek_badge(json) };
}

fn format_countdown(seconds: u64) -> String {
    if seconds == 0 {
        return "now".to_string();
    }
    let minutes = (seconds + 59) / 60; // ceil
    if minutes < 60 {
        format!("~{minutes} min")
    } else {
        let hours = minutes / 60;
        let remainder = minutes % 60;
        if remainder == 0 {
            format!("~{hours} hr")
        } else {
            format!("~{hours} hr {remainder} min")
        }
    }
}

fn notify(title: &str, body: &str) {
    let _ = unsafe {
        peekoo_notify(Json(NotifyRequest {
            title: title.to_string(),
            body: body.to_string(),
        }))
    };
}
