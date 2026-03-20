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
    #[serde(default)]
    event_reminders: Vec<EventReminder>,
}

#[derive(Clone, Serialize, Deserialize)]
struct EventReminder {
    event_name: String,
    message: String,
    created_at: u64,
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
                if key.ends_with(":pre") {
                    // Pre-events only trigger a badge refresh, which happens at the end of this function
                } else {
                    handle_schedule_fired(key);
                }
            }
        }
        "system:wake" => {
            sync_schedules();
        }
        _ => {
            handle_custom_event(event_name);
        }
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
pub fn tool_health_add_event_reminder(input: String) -> FnResult<String> {
    let reminder: EventReminder = serde_json::from_str(&input)?;
    let mut reminders = load_event_reminders();
    reminders.push(EventReminder {
        created_at: current_epoch_secs(),
        ..reminder
    });
    save_event_reminders(&reminders);
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

    let mut reminders = Vec::new();
    if config.water_enabled {
        reminders.push((WATER_KEY, u64::from(config.water_interval_min) * 60));
    }
    if config.eye_rest_enabled {
        reminders.push((EYE_REST_KEY, u64::from(config.eye_rest_interval_min) * 60));
    }
    if config.standup_enabled {
        reminders.push((STANDUP_KEY, u64::from(config.standup_interval_min) * 60));
    }

    let now = current_epoch_secs();
    for (key, interval_secs) in reminders {
        let delay = compute_remaining_delay(key, interval_secs, now, false);
        schedule_set_with_delay(key, interval_secs, delay);

        // Schedule pre-event 60s before the primary reminder
        let pre_threshold = 60;
        if interval_secs > pre_threshold {
            let pre_key = format!("{}:pre", key);
            let effective_delay = delay.unwrap_or(interval_secs);
            let pre_delay = if effective_delay > pre_threshold {
                Some(effective_delay - pre_threshold)
            } else {
                Some(0)
            };
            // Note: pre-events use the same interval but fire earlier
            let _ = peekoo::schedule::set(&pre_key, interval_secs, true, pre_delay);
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
        schedule_cancel(&format!("{}:pre", key));
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
        event_reminders: load_event_reminders(),
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
        water_interval_min: state_get("water_interval_min")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| config["water_interval_min"].as_u64().unwrap_or(45)) as u32,
        water_enabled: state_get("water_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or_else(|| config["water_enabled"].as_bool().unwrap_or(true)),
        eye_rest_interval_min: state_get("eye_rest_interval_min")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| config["eye_rest_interval_min"].as_u64().unwrap_or(20)) as u32,
        eye_rest_enabled: state_get("eye_rest_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or_else(|| config["eye_rest_enabled"].as_bool().unwrap_or(true)),
        standup_interval_min: state_get("standup_interval_min")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| config["standup_interval_min"].as_u64().unwrap_or(60)) as u32,
        standup_enabled: state_get("standup_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or_else(|| config["standup_enabled"].as_bool().unwrap_or(true)),
    }
}

fn save_config(config: &ReminderConfig) {
    state_set("water_interval_min", json!(config.water_interval_min));
    state_set("water_enabled", json!(config.water_enabled));
    state_set("eye_rest_interval_min", json!(config.eye_rest_interval_min));
    state_set("eye_rest_enabled", json!(config.eye_rest_enabled));
    state_set("standup_interval_min", json!(config.standup_interval_min));
    state_set("standup_enabled", json!(config.standup_enabled));
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

fn load_event_reminders() -> Vec<EventReminder> {
    state_get("event_reminders")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn save_event_reminders(reminders: &[EventReminder]) {
    state_set("event_reminders", json!(reminders));
}

fn handle_custom_event(event_name: &str) {
    let mut reminders = load_event_reminders();
    let original_count = reminders.len();
    
    // Find matching reminders
    let (to_trigger, to_keep): (Vec<_>, Vec<_>) = reminders
        .into_iter()
        .partition(|r| r.event_name == event_name);
    
    for r in to_trigger {
        notify("Linked Reminder", &r.message);
    }
    
    if to_keep.len() != original_count {
        save_event_reminders(&to_keep);
    }
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
    let threshold_secs = 60; // Only show badge 60s before due

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
        .filter(|reminder| reminder.active && reminder.time_remaining_secs <= threshold_secs)
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
            target_epoch_secs: Some(current_epoch_secs() + reminder.time_remaining_secs),
        })
        .collect();

    let _ = peekoo::badge::set(&items);
}

fn format_countdown(seconds: u64) -> String {
    if seconds <= 60 {
        let mins = seconds / 60;
        let secs = seconds % 60;
        return format!("{:02}:{:02}", mins, secs);
    }
    let minutes = seconds / 60;
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
    let _ = peekoo::notify::send(title, body);
}
