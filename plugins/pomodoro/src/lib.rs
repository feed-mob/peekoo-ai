#![no_main]

use peekoo_plugin_sdk::prelude::*;
use serde_json::{json, Value};

const POMODORO_TIMER_KEY: &str = "pomodoro_timer";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PomodoroState {
    Idle,
    Running,
    Paused,
    Completed,
}

#[derive(Clone, Serialize, Deserialize)]
struct PomodoroSession {
    mode: String,
    state: PomodoroState,
    minutes: u32,
    time_remaining_secs: u64,
    started_at_epoch: u64,
    expected_fire_at_epoch: u64,
    #[serde(default)]
    completed_focus: u32,
    #[serde(default)]
    completed_breaks: u32,
    #[serde(default)]
    enable_memo: bool,
    #[serde(default)]
    memos: Vec<Value>,
    #[serde(default)]
    last_date: String,
    // Configuration
    #[serde(default)]
    default_work_minutes: u32,
    #[serde(default)]
    default_break_minutes: u32,
}

impl PomodoroSession {
    fn unwrap_or_default_session() -> Self {
        Self {
            mode: "work".to_string(),
            state: PomodoroState::Idle,
            minutes: 25,
            time_remaining_secs: 25 * 60,
            started_at_epoch: 0,
            expected_fire_at_epoch: 0,
            completed_focus: 0,
            completed_breaks: 0,
            enable_memo: false,
            memos: Vec::new(),
            last_date: String::new(),
            default_work_minutes: 25,
            default_break_minutes: 5,
        }
    }
}

#[plugin_fn]
pub fn plugin_init(_input: String) -> FnResult<String> {
    log_info("Pomodoro plugin initialized");
    if get_session().is_none() {
        save_session(&PomodoroSession::unwrap_or_default_session());
    }
    sync_schedule();
    push_peek_badges();
    Ok(json!({ "status": "ok" }).to_string())
}

#[plugin_fn]
pub fn on_event(input: String) -> FnResult<String> {
    let event: Value = serde_json::from_str(&input)?;
    let event_name = event["event"].as_str().unwrap_or("");
    
    match event_name {
        "schedule:fired" => {
            if let Some(key) = event["payload"]["key"].as_str() {
                if key == POMODORO_TIMER_KEY {
                    handle_timer_completed();
                }
            }
        }
        "system:wake" => {
            sync_schedule();
        }
        _ => {}
    }
    push_peek_badges();
    Ok(json!({ "ok": true }).to_string())
}

#[plugin_fn]
pub fn tool_pomodoro_get_status(_input: String) -> FnResult<String> {
    let mut session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
    
    // Auto-complete if time up during a status check
    if session.state == PomodoroState::Running {
        refresh_time_remaining(&mut session);
        if session.time_remaining_secs == 0 {
            handle_timer_completed();
            session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
        }
    } else {
        refresh_time_remaining(&mut session);
    }
    
    // Explicitly return serialized JSON
    Ok(serde_json::to_string(&session)?)
}

#[derive(Deserialize)]
struct StartInput {
    mode: String,
    minutes: u32,
}

#[plugin_fn]
pub fn tool_pomodoro_start(input: String) -> FnResult<String> {
    let params: StartInput = serde_json::from_str(&input)?;
    let mut session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
    
    session.mode = params.mode;
    session.minutes = params.minutes;
    session.time_remaining_secs = session.minutes as u64 * 60;
    session.state = PomodoroState::Running;
    session.started_at_epoch = current_epoch_secs();
    session.expected_fire_at_epoch = session.started_at_epoch + session.time_remaining_secs;
    
    save_session(&session);
    sync_schedule();
    
    if session.mode == "work" {
        let _ = peekoo::mood::set("pomodoro-started", true);
    } else {
        let _ = peekoo::mood::set("pomodoro-resting", true);
    }
    
    emit_event("pomodoro:started", json!({ "mode": session.mode }));
    push_peek_badges();
    Ok(serde_json::to_string(&session)?)
}

#[plugin_fn]
pub fn tool_pomodoro_pause(_input: String) -> FnResult<String> {
    let mut session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
    if session.state == PomodoroState::Running {
        refresh_time_remaining(&mut session);
        session.state = PomodoroState::Paused;
        save_session(&session);
        cancel_schedule();
        let _ = peekoo::mood::set("pomodoro-break", false);
        emit_event("pomodoro:paused", json!({}));
    }
    push_peek_badges();
    Ok(serde_json::to_string(&session)?)
}

#[plugin_fn]
pub fn tool_pomodoro_resume(_input: String) -> FnResult<String> {
    let mut session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
    if session.state == PomodoroState::Paused {
        session.state = PomodoroState::Running;
        session.expected_fire_at_epoch = current_epoch_secs() + session.time_remaining_secs;
        save_session(&session);
        sync_schedule();
        
        if session.mode == "work" {
            let _ = peekoo::mood::set("pomodoro-started", true);
        } else {
            let _ = peekoo::mood::set("pomodoro-resting", true);
        }
        
        emit_event("pomodoro:resumed", json!({}));
    }
    push_peek_badges();
    Ok(serde_json::to_string(&session)?)
}

#[plugin_fn]
pub fn tool_pomodoro_finish(_input: String) -> FnResult<String> {
    let mut session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
    session.state = PomodoroState::Idle;
    refresh_time_remaining(&mut session);
    session.time_remaining_secs = session.minutes as u64 * 60;
    
    save_session(&session);
    cancel_schedule();
    let _ = peekoo::mood::set("pomodoro-break", false);
    emit_event("pomodoro:finished", json!({}));
    push_peek_badges();
    Ok(serde_json::to_string(&session)?)
}

#[derive(Deserialize)]
struct SettingsInput {
    work_minutes: u32,
    break_minutes: u32,
    enable_memo: Option<bool>,
}

#[plugin_fn]
pub fn tool_pomodoro_set_settings(input: String) -> FnResult<String> {
    let params: SettingsInput = serde_json::from_str(&input)?;
    let mut session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
    
    session.default_work_minutes = params.work_minutes;
    session.default_break_minutes = params.break_minutes;
    if let Some(enable) = params.enable_memo {
        log_info(&format!("Setting enable_memo to: {}", enable));
        session.enable_memo = enable;
    }
    
    log_info(&format!("Final session enable_memo: {}", session.enable_memo));
    
    if session.state == PomodoroState::Idle {
        session.minutes = if session.mode == "work" {
            session.default_work_minutes
        } else {
            session.default_break_minutes
        };
        session.time_remaining_secs = session.minutes as u64 * 60;
    }
    
    save_session(&session);
    push_peek_badges();
    Ok(serde_json::to_string(&session)?)
}

#[plugin_fn]
pub fn tool_pomodoro_add_memo(input: String) -> FnResult<String> {
    let memo_content: String = serde_json::from_str(&input)?;
    let mut session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
    
    session.memos.push(json!({
        "timestamp": current_epoch_secs(),
        "content": memo_content,
        "mode": session.mode
    }));
    
    log_info("New memo added to session.");
    save_session(&session);
    Ok(json!({ "ok": true }).to_string())
}

// ---------------------------------------------------------
// Helper functions
// ---------------------------------------------------------

fn get_session() -> Option<PomodoroSession> {
    peekoo::state::get::<PomodoroSession>("pomodoro_session").ok().flatten()
}

fn save_session(session: &PomodoroSession) {
    let _ = peekoo::state::set("pomodoro_session", session);
}

fn current_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn refresh_time_remaining(session: &mut PomodoroSession) {
    if session.state == PomodoroState::Running {
        let now = current_epoch_secs();
        if now >= session.expected_fire_at_epoch {
            session.time_remaining_secs = 0;
        } else {
            session.time_remaining_secs = session.expected_fire_at_epoch - now;
        }
    }
}

fn handle_timer_completed() {
    let mut session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
    session.state = PomodoroState::Completed;
    session.time_remaining_secs = 0;
    
    let title = if session.mode == "work" {
        "Focus Session Complete"
    } else {
        "Break Complete"
    };
    
    let body = if session.mode == "work" {
        session.completed_focus += 1;
        "Great job! Time to take a short break."
    } else {
        session.completed_breaks += 1;
        "Ready to start focusing again?"
    };

    save_session(&session);
    cancel_schedule();
    
    let _ = peekoo::mood::set("pomodoro-completed", false);
    let _ = peekoo::notify::send(title, body);
    emit_event("pomodoro:completed", json!({ "mode": session.mode, "completed_focus": session.completed_focus, "completed_breaks": session.completed_breaks }));
    push_peek_badges();
}

fn sync_schedule() {
    let mut session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
    if session.state == PomodoroState::Running {
        refresh_time_remaining(&mut session);
        if session.time_remaining_secs > 0 {
            let _ = peekoo::schedule::set(POMODORO_TIMER_KEY, session.time_remaining_secs, false, None);
        } else {
            handle_timer_completed();
        }
    }
}

fn cancel_schedule() {
    let _ = peekoo::schedule::cancel(POMODORO_TIMER_KEY);
}

fn push_peek_badges() {
    let mut session = get_session().unwrap_or(PomodoroSession::unwrap_or_default_session());
    refresh_time_remaining(&mut session);

    if session.state == PomodoroState::Running || session.state == PomodoroState::Paused {
        let icon = if session.mode == "work" { "brain" } else { "coffee" };
        let label = if session.state == PomodoroState::Paused {
            if session.mode == "work" { "Focus (Paused)".to_string() } else { "Break (Paused)".to_string() }
        } else {
            if session.mode == "work" { "Focus".to_string() } else { "Break".to_string() }
        };

        let countdown = if session.state == PomodoroState::Running {
            Some(session.time_remaining_secs)
        } else {
            None
        };

        let badge = BadgeItem {
            label,
            value: format_countdown(session.time_remaining_secs),
            icon: Some(icon.to_string()),
            countdown_secs: countdown,
        };
        let _ = peekoo::badge::set(&[badge]);
    } else {
        let _ = peekoo::badge::set(&[]);
    }
}

fn format_countdown(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", mins, secs)
}

fn log_info(message: &str) {
    peekoo::log::info(message);
}

fn emit_event(event: &str, payload: Value) {
    let _ = peekoo::events::emit(event, payload);
}
