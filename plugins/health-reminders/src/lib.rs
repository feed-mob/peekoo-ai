#![no_main]

use peekoo_plugin_sdk::prelude::*;
use serde_json::{json, Value};

const WATER_KEY: &str = "water";
const EYE_REST_KEY: &str = "eye_rest";
const STANDUP_KEY: &str = "standup";

#[derive(Clone, Serialize, Deserialize)]
struct ReminderConfig {
    water_interval_min: u32,
    water_enabled: bool,
    eye_rest_interval_min: u32,
    eye_rest_enabled: bool,
    standup_interval_min: u32,
    standup_enabled: bool,
    global_enabled: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct ReminderState {
    reminder_type: String,
    interval_min: u32,
    time_remaining_secs: u64,
    active: bool,
    enabled: bool,
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
    let old_config = load_config();
    let mut config = old_config.clone();

    if let Some(value) = patch["water_interval_min"].as_u64() {
        config.water_interval_min = (value as u32).clamp(5, 180);
    }
    if let Some(value) = patch["water_enabled"].as_bool() {
        config.water_enabled = value;
    }
    if let Some(value) = patch["eye_rest_interval_min"].as_u64() {
        config.eye_rest_interval_min = (value as u32).clamp(5, 120);
    }
    if let Some(value) = patch["eye_rest_enabled"].as_bool() {
        config.eye_rest_enabled = value;
    }
    if let Some(value) = patch["standup_interval_min"].as_u64() {
        config.standup_interval_min = (value as u32).clamp(10, 180);
    }
    if let Some(value) = patch["standup_enabled"].as_bool() {
        config.standup_enabled = value;
    }
    if let Some(value) = patch["global_enabled"].as_bool() {
        config.global_enabled = value;
    }

    save_config(&config);
    
    // Only reset schedules if critical values changed. 
    // This prevents timer stalling on every UI poll/update.
    if config.global_enabled != old_config.global_enabled 
       || config.water_enabled != old_config.water_enabled
       || config.eye_rest_enabled != old_config.eye_rest_enabled
       || config.standup_enabled != old_config.standup_enabled
       || config.water_interval_min != old_config.water_interval_min
       || config.eye_rest_interval_min != old_config.eye_rest_interval_min
       || config.standup_interval_min != old_config.standup_interval_min
    {
        sync_schedules();
    }

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
    let config = load_config();
    cancel_all_schedules();

    if !config.global_enabled {
        return;
    }

    let reminders = [
        (WATER_KEY, u64::from(config.water_interval_min) * 60, config.water_enabled),
        (EYE_REST_KEY, u64::from(config.eye_rest_interval_min) * 60, config.eye_rest_enabled),
        (STANDUP_KEY, u64::from(config.standup_interval_min) * 60, config.standup_enabled),
    ];

    let now = current_epoch_secs();
    for (key, interval_secs, enabled) in reminders {
        if enabled {
            let delay = compute_remaining_delay(key, interval_secs, now, false);
            schedule_set_with_delay(key, interval_secs, delay);
        }
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
            load_reminder_state(WATER_KEY, config.water_interval_min, config.water_enabled),
            load_reminder_state(EYE_REST_KEY, config.eye_rest_interval_min, config.eye_rest_enabled),
            load_reminder_state(STANDUP_KEY, config.standup_interval_min, config.standup_enabled),
        ],
    }
}

fn load_reminder_state(reminder_type: &str, interval_min: u32, enabled: bool) -> ReminderState {
    let schedule = schedule_get(reminder_type);
    ReminderState {
        reminder_type: reminder_type.to_string(),
        interval_min,
        time_remaining_secs: schedule
            .as_ref()
            .map(|value| value.time_remaining_secs)
            .unwrap_or(0),
        active: schedule.is_some(),
        enabled,
    }
}

fn load_config() -> ReminderConfig {
    let config = config_get();
    ReminderConfig {
        water_interval_min: config["water_interval_min"].as_u64().unwrap_or(45) as u32,
        water_enabled: config["water_enabled"].as_bool().unwrap_or(true),
        eye_rest_interval_min: config["eye_rest_interval_min"].as_u64().unwrap_or(20) as u32,
        eye_rest_enabled: config["eye_rest_enabled"].as_bool().unwrap_or(true),
        standup_interval_min: config["standup_interval_min"].as_u64().unwrap_or(60) as u32,
        standup_enabled: config["standup_enabled"].as_bool().unwrap_or(true),
        global_enabled: config["global_enabled"].as_bool().unwrap_or(true),
    }
}

fn save_config(config: &ReminderConfig) {
    let _ = peekoo::state::set("water_interval_min", &config.water_interval_min);
    let _ = peekoo::state::set("water_enabled", &config.water_enabled);
    let _ = peekoo::state::set("eye_rest_interval_min", &config.eye_rest_interval_min);
    let _ = peekoo::state::set("eye_rest_enabled", &config.eye_rest_enabled);
    let _ = peekoo::state::set("standup_interval_min", &config.standup_interval_min);
    let _ = peekoo::state::set("standup_enabled", &config.standup_enabled);
    let _ = peekoo::state::set("global_enabled", &config.global_enabled);
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
    peekoo::config::get_all().ok().unwrap_or_else(|| json!({}))
}

fn schedule_get(key: &str) -> Option<ScheduleInfo> {
    peekoo::schedule::get(key).ok().flatten()
}

fn schedule_set(key: &str, interval_secs: u64) {
    schedule_set_with_delay(key, interval_secs, None);
}

fn schedule_set_with_delay(key: &str, interval_secs: u64, delay_secs: Option<u64>) {
    let _ = peekoo::schedule::set(key, interval_secs, true, delay_secs);
    save_timer_started_at(key, interval_secs, delay_secs);
}

fn schedule_cancel(key: &str) {
    let _ = peekoo::schedule::cancel(key);
}

fn state_get(key: &str) -> Option<Value> {
    peekoo::state::get::<Value>(key).ok().flatten()
}

fn state_set(key: &str, value: Value) {
    let _ = peekoo::state::set(key, &value);
}

fn log_info(message: &str) {
    peekoo::log::info(message);
}

fn emit_event(event: &str, payload: Value) {
    let _ = peekoo::events::emit(event, payload);
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

    let items: Vec<BadgeItem> = status
        .reminders
        .iter()
        .filter(|reminder| reminder.active)
        .map(|reminder| BadgeItem {
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

    let _ = peekoo::badge::set(&items);
}

fn format_countdown(seconds: u64) -> String {
    if seconds == 0 {
        return "00:00".to_string();
    }
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", mins, secs)
}

fn notify(title: &str, body: &str) {
    let _ = peekoo::notify::send(title, body);
}
