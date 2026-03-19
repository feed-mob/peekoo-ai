#![cfg_attr(target_arch = "wasm32", no_main)]

use peekoo_plugin_sdk::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Bridge file schema ─────────────────────────────────────────

#[derive(Deserialize, Default)]
struct BridgeState {
    status: Option<String>,
    session_title: Option<String>,
    started_at: Option<u64>,
    sessions: Option<Vec<BridgeSession>>,
    completed_sessions: Option<Vec<CompletedSession>>,
    #[allow(dead_code)]
    updated_at: Option<u64>,
}

#[derive(Clone, Deserialize, Default)]
struct BridgeSession {
    #[allow(dead_code)]
    session_id: Option<String>,
    status: Option<String>,
    session_title: Option<String>,
    #[allow(dead_code)]
    started_at: Option<u64>,
    #[allow(dead_code)]
    updated_at: Option<u64>,
}

#[derive(Clone, Deserialize, Default)]
struct CompletedSession {
    completion_id: Option<String>,
    #[allow(dead_code)]
    session_id: Option<String>,
    session_title: Option<String>,
    #[allow(dead_code)]
    updated_at: Option<u64>,
}

// ── Constants ──────────────────────────────────────────────────

const SCHEDULE_KEY: &str = "poll-opencode";
const POLL_INTERVAL_SECS: u64 = 2;
const STATE_LAST_STATUS: &str = "last_status";
const STATE_SEEN_COMPLETIONS: &str = "seen_completed_sessions";
const MAX_TRACKED_COMPLETIONS: usize = 32;
const MAX_DISPLAY_LEN: usize = 30;
const STALE_SESSION_TIMEOUT_SECS: u64 = 30;

fn truncate_title(s: &str) -> String {
    if s.chars().count() > MAX_DISPLAY_LEN {
        format!(
            "{}...",
            s.chars().take(MAX_DISPLAY_LEN - 3).collect::<String>()
        )
    } else {
        s.to_string()
    }
}

fn display_badge_title(title: Option<&str>) -> String {
    match title.map(str::trim) {
        Some(title) if !title.is_empty() => truncate_title(title),
        _ => "Working...".to_string(),
    }
}

fn session_badge_value(status: Option<&str>) -> String {
    match status {
        Some("waiting") => "Needs input".to_string(),
        Some("thinking") => "Thinking".to_string(),
        _ => "Working".to_string(),
    }
}

fn completion_id(session: &CompletedSession) -> Option<String> {
    session
        .completion_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn completed_session_notification_title(session: &CompletedSession) -> Option<String> {
    session
        .session_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(truncate_title)
}

fn completed_sessions(state: &BridgeState) -> &[CompletedSession] {
    state.completed_sessions.as_deref().unwrap_or_default()
}

fn trim_seen_completions(seen: &mut Vec<String>) {
    if seen.len() > MAX_TRACKED_COMPLETIONS {
        let drain_count = seen.len() - MAX_TRACKED_COMPLETIONS;
        seen.drain(0..drain_count);
    }
}

fn should_refresh_working_mood(current_status: &str, new_completion_count: usize) -> bool {
    matches!(current_status, "working" | "thinking") && new_completion_count == 0
}

fn mood_trigger_for_status(status: &str) -> &'static str {
    match status {
        "working" | "thinking" => "working",
        "waiting" => "reminder",
        "happy" | "done" => "happy",
        _ => "idle",
    }
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn is_interruptible_status(status: &str) -> bool {
    matches!(status, "working" | "thinking" | "waiting")
}

fn is_stale_active_state(state: &BridgeState, now: u64) -> bool {
    let Some(status) = state.status.as_deref() else {
        return false;
    };

    if !is_interruptible_status(status) {
        return false;
    }

    let Some(updated_at) = state.updated_at else {
        return false;
    };

    now.saturating_sub(updated_at) >= STALE_SESSION_TIMEOUT_SECS
}

fn active_sessions(state: &BridgeState) -> Vec<BridgeSession> {
    let mut sessions = state.sessions.clone().unwrap_or_default();
    sessions.retain(|session| {
        matches!(
            session.status.as_deref(),
            Some("working") | Some("thinking") | Some("waiting")
        )
    });
    sessions
}

fn distinct_badge_labels(sessions: &[BridgeSession]) -> Vec<String> {
    let base_labels: Vec<String> = sessions
        .iter()
        .map(|session| display_badge_title(session.session_title.as_deref()))
        .collect();

    let mut counts = std::collections::HashMap::<String, usize>::new();
    for label in &base_labels {
        *counts.entry(label.clone()).or_insert(0) += 1;
    }

    let mut seen = std::collections::HashMap::<String, usize>::new();
    base_labels
        .into_iter()
        .map(|label| {
            let total = counts.get(&label).copied().unwrap_or(0);
            if total <= 1 {
                return label;
            }

            let ordinal = seen.entry(label.clone()).or_insert(0);
            *ordinal += 1;
            format!("{} ({})", label, ordinal)
        })
        .collect()
}

fn badge_items_from_state(state: &BridgeState) -> Vec<BadgeItem> {
    let sessions = active_sessions(state);
    if sessions.len() > 1 {
        let labels = distinct_badge_labels(&sessions);
        return sessions
            .into_iter()
            .zip(labels)
            .map(|(session, label)| BadgeItem {
                label,
                value: session_badge_value(session.status.as_deref()),
                icon: Some("activity".into()),
                countdown_secs: None,
            })
            .collect();
    }

    let display_title = display_badge_title(state.session_title.as_deref());

    vec![BadgeItem {
        label: "OpenCode".into(),
        value: display_title,
        icon: Some("activity".into()),
        countdown_secs: None,
    }]
}

// ── Lifecycle ──────────────────────────────────────────────────

#[plugin_fn]
pub fn plugin_init(_: String) -> FnResult<String> {
    peekoo::log::info("OpenCode Companion: initializing");

    // Set up polling schedule
    peekoo::schedule::set(SCHEDULE_KEY, POLL_INTERVAL_SECS, true, None)?;

    // Start with idle state
    peekoo::state::set(STATE_LAST_STATUS, &"idle".to_string())?;

    // Clear any stale badges
    peekoo::badge::set(&[])?;

    Ok(r#"{"status":"ok"}"#.into())
}

// ── Event handler ──────────────────────────────────────────────

#[derive(Deserialize)]
struct EventInput {
    event: String,
    payload: Value,
}

#[plugin_fn]
pub fn on_event(Json(input): Json<EventInput>) -> FnResult<String> {
    if input.event != "schedule:fired" {
        return Ok(r#"{"ok":true}"#.into());
    }

    // Only handle our own schedule
    let fired_key = input.payload["key"].as_str().unwrap_or("");
    if fired_key != SCHEDULE_KEY {
        return Ok(r#"{"ok":true}"#.into());
    }

    poll_bridge()?;
    Ok(r#"{"ok":true}"#.into())
}

// ── Core logic ─────────────────────────────────────────────────

fn poll_bridge() -> Result<(), extism_pdk::Error> {
    let bridge_data = peekoo::bridge::read()?;

    let state: BridgeState = match bridge_data {
        Some(ref contents) if !contents.is_empty() => {
            serde_json::from_str(contents).unwrap_or_default()
        }
        _ => BridgeState::default(),
    };

    let now = current_unix_timestamp();
    let is_stale = is_stale_active_state(&state, now);
    let current_status = if is_stale {
        "idle"
    } else {
        state.status.as_deref().unwrap_or("idle")
    };
    let previous_status: String =
        peekoo::state::get(STATE_LAST_STATUS)?.unwrap_or_else(|| "idle".to_string());
    let mut seen_completions: Vec<String> =
        peekoo::state::get(STATE_SEEN_COMPLETIONS)?.unwrap_or_default();
    let mut seen_set: std::collections::HashSet<String> =
        seen_completions.iter().cloned().collect();
    let mut new_completion_count = 0usize;

    if !is_stale {
        for completion in completed_sessions(&state) {
            let Some(id) = completion_id(&completion) else {
                continue;
            };

            if !seen_set.insert(id.clone()) {
                continue;
            }

            handle_completed_session(&completion)?;
            seen_completions.push(id);
            new_completion_count += 1;
        }
    }

    trim_seen_completions(&mut seen_completions);
    peekoo::state::set(STATE_SEEN_COMPLETIONS, &seen_completions)?;

    // Only act on status changes
    if current_status != previous_status {
        handle_status_change(current_status, &state, new_completion_count)?;
        peekoo::state::set(STATE_LAST_STATUS, &current_status.to_string())?;
    }

    // Always update badge when active (elapsed time changes)
    if should_refresh_working_mood(current_status, new_completion_count) {
        update_badge(&state)?;
    }

    Ok(())
}

fn handle_completed_session(session: &CompletedSession) -> Result<(), extism_pdk::Error> {
    peekoo::mood::set(mood_trigger_for_status("done"), false)?;

    let body = match completed_session_notification_title(session) {
        Some(title) => format!("🎉 {} is done!", title),
        None => "🎉 OpenCode has finished working".to_string(),
    };
    let _ = peekoo::notify::send("OpenCode", &body);

    Ok(())
}

fn handle_status_change(
    new_status: &str,
    state: &BridgeState,
    new_completion_count: usize,
) -> Result<(), extism_pdk::Error> {
    match new_status {
        "working" => {
            if should_refresh_working_mood(new_status, new_completion_count) {
                peekoo::mood::set(mood_trigger_for_status(new_status), true)?;
            }
            update_badge(state)?;
        }
        "thinking" => {
            if should_refresh_working_mood(new_status, new_completion_count) {
                peekoo::mood::set(mood_trigger_for_status(new_status), true)?;
            }
            update_badge(state)?;
        }
        "waiting" => {
            peekoo::mood::set(mood_trigger_for_status(new_status), false)?;
            let _ = peekoo::notify::send("OpenCode", "OpenCode needs your input");
            update_badge(state)?;
        }
        "happy" | "done" => {
            if completed_sessions(state).is_empty() {
                let fallback = CompletedSession {
                    completion_id: Some("status-change".to_string()),
                    session_id: None,
                    session_title: state.session_title.clone(),
                    updated_at: state.updated_at,
                };
                handle_completed_session(&fallback)?;
            }

            peekoo::badge::set(&[])?;
        }
        _ => {
            // "idle" or unknown
            peekoo::mood::set(mood_trigger_for_status(new_status), false)?;
            peekoo::badge::set(&[])?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── truncate_title ─────────────────────────────────────────

    #[test]
    fn truncate_title_empty_string() {
        assert_eq!(truncate_title(""), "");
    }

    #[test]
    fn truncate_title_short_ascii() {
        assert_eq!(truncate_title("Fix bug"), "Fix bug");
    }

    #[test]
    fn truncate_title_exactly_at_limit() {
        // 30 chars exactly — no truncation
        let title = "a".repeat(MAX_DISPLAY_LEN);
        assert_eq!(truncate_title(&title), title);
    }

    #[test]
    fn truncate_title_one_over_limit() {
        // 31 chars — should truncate to 27 chars + "..."
        let title = "a".repeat(MAX_DISPLAY_LEN + 1);
        let result = truncate_title(&title);
        assert_eq!(result, format!("{}...", "a".repeat(MAX_DISPLAY_LEN - 3)));
        assert_eq!(result.chars().count(), MAX_DISPLAY_LEN);
    }

    #[test]
    fn truncate_title_long_ascii() {
        let title = "Help me write a new feature that allows users to track metrics";
        let result = truncate_title(title);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), MAX_DISPLAY_LEN);
    }

    #[test]
    fn truncate_title_multibyte_emoji() {
        // Emoji are multibyte in UTF-8 — this would panic with byte indexing
        let title = "🚀".repeat(MAX_DISPLAY_LEN + 1);
        let result = truncate_title(&title);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), MAX_DISPLAY_LEN);
    }

    #[test]
    fn truncate_title_mixed_emoji_and_ascii() {
        // Mix of ASCII and multibyte chars exceeding the limit
        let title = "Fix 🐛 in the 🎨 rendering pipeline for 🚀 deployment";
        let result = truncate_title(title);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), MAX_DISPLAY_LEN);
    }

    #[test]
    fn truncate_title_cjk_characters() {
        // CJK chars are 3 bytes each in UTF-8 — byte indexing would panic
        let title = "日本語のタスク名前がとても長い場合のテストケースですから確認する";
        assert!(title.chars().count() > MAX_DISPLAY_LEN);
        let result = truncate_title(title);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), MAX_DISPLAY_LEN);
    }

    #[test]
    fn truncate_title_emoji_at_exact_limit() {
        // Exactly 30 emoji chars — no truncation needed
        let title = "🎉".repeat(MAX_DISPLAY_LEN);
        assert_eq!(truncate_title(&title), title);
    }

    #[test]
    fn display_badge_title_falls_back_for_empty_title() {
        assert_eq!(display_badge_title(Some("")), "Working...");
    }

    #[test]
    fn display_badge_title_uses_non_empty_title() {
        assert_eq!(display_badge_title(Some("Fix badge")), "Fix badge");
    }

    #[test]
    fn badge_items_from_state_rotates_multiple_active_sessions() {
        let state = BridgeState {
            status: Some("working".to_string()),
            session_title: Some("Second session".to_string()),
            started_at: Some(10),
            completed_sessions: None,
            updated_at: Some(12),
            sessions: Some(vec![
                BridgeSession {
                    session_id: Some("session-2".to_string()),
                    status: Some("working".to_string()),
                    session_title: Some("Second session".to_string()),
                    started_at: Some(11),
                    updated_at: Some(12),
                },
                BridgeSession {
                    session_id: Some("session-1".to_string()),
                    status: Some("thinking".to_string()),
                    session_title: Some("First session".to_string()),
                    started_at: Some(10),
                    updated_at: Some(11),
                },
            ]),
        };

        let badges = badge_items_from_state(&state);
        assert_eq!(badges.len(), 2);
        assert_eq!(badges[0].label, "Second session");
        assert_eq!(badges[0].value, "Working");
        assert_eq!(badges[1].label, "First session");
        assert_eq!(badges[1].value, "Thinking");
    }

    #[test]
    fn distinct_badge_labels_disambiguates_duplicate_titles() {
        let sessions = vec![
            BridgeSession {
                session_id: Some("session-1".to_string()),
                status: Some("working".to_string()),
                session_title: Some("OpenCode session".to_string()),
                started_at: Some(1),
                updated_at: Some(2),
            },
            BridgeSession {
                session_id: Some("session-2".to_string()),
                status: Some("thinking".to_string()),
                session_title: Some("OpenCode session".to_string()),
                started_at: Some(3),
                updated_at: Some(4),
            },
        ];

        assert_eq!(
            distinct_badge_labels(&sessions),
            vec![
                "OpenCode session (1)".to_string(),
                "OpenCode session (2)".to_string()
            ]
        );
    }

    #[test]
    fn completed_session_marker_can_coexist_with_active_sessions() {
        let state = BridgeState {
            status: Some("working".to_string()),
            session_title: Some("Keep B running".to_string()),
            started_at: Some(20),
            completed_sessions: None,
            sessions: Some(vec![BridgeSession {
                session_id: Some("session-b".to_string()),
                status: Some("working".to_string()),
                session_title: Some("Keep B running".to_string()),
                started_at: Some(20),
                updated_at: Some(21),
            }]),
            updated_at: Some(21),
        };

        let badges = badge_items_from_state(&state);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].label, "OpenCode");
        assert_eq!(badges[0].value, "Keep B running");
    }

    #[test]
    fn completion_id_uses_explicit_completion_identifier() {
        let completed = CompletedSession {
            completion_id: Some("session-a:42:0".to_string()),
            session_id: Some("session-a".to_string()),
            session_title: Some("Finish A".to_string()),
            updated_at: Some(42),
        };

        assert_eq!(completion_id(&completed).as_deref(), Some("session-a:42:0"));
    }

    #[test]
    fn completed_session_notification_title_uses_truncated_title() {
        let completed = CompletedSession {
            completion_id: Some("session-a:42:0".to_string()),
            session_id: Some("session-a".to_string()),
            session_title: Some("Finish a very long task title that needs truncating".to_string()),
            updated_at: Some(42),
        };

        let title = completed_session_notification_title(&completed).unwrap();
        assert!(title.ends_with("..."));
        assert_eq!(title.chars().count(), MAX_DISPLAY_LEN);
    }

    #[test]
    fn completed_sessions_returns_all_queued_completions() {
        let state = BridgeState {
            status: Some("happy".to_string()),
            session_title: Some("Done".to_string()),
            started_at: Some(0),
            sessions: Some(vec![]),
            completed_sessions: Some(vec![
                CompletedSession {
                    completion_id: Some("session-a:42:0".to_string()),
                    session_id: Some("session-a".to_string()),
                    session_title: Some("First".to_string()),
                    updated_at: Some(42),
                },
                CompletedSession {
                    completion_id: Some("session-b:43:1".to_string()),
                    session_id: Some("session-b".to_string()),
                    session_title: Some("Second".to_string()),
                    updated_at: Some(43),
                },
            ]),
            updated_at: Some(43),
        };

        let completions = completed_sessions(&state);
        assert_eq!(completions.len(), 2);
        assert_eq!(
            completion_id(&completions[0]).as_deref(),
            Some("session-a:42:0")
        );
        assert_eq!(
            completion_id(&completions[1]).as_deref(),
            Some("session-b:43:1")
        );
    }

    #[test]
    fn working_mood_is_suppressed_when_new_completion_arrives() {
        assert!(!should_refresh_working_mood("working", 1));
        assert!(!should_refresh_working_mood("thinking", 2));
    }

    #[test]
    fn working_mood_resumes_when_no_new_completion_arrives() {
        assert!(should_refresh_working_mood("working", 0));
        assert!(should_refresh_working_mood("thinking", 0));
        assert!(!should_refresh_working_mood("happy", 0));
    }

    #[test]
    fn waiting_status_uses_needs_input_badge_value() {
        assert_eq!(session_badge_value(Some("waiting")), "Needs input");
    }

    #[test]
    fn waiting_status_does_not_refresh_working_mood() {
        assert!(!should_refresh_working_mood("waiting", 0));
    }

    #[test]
    fn mood_trigger_for_active_statuses_uses_canonical_sprite_names() {
        assert_eq!(mood_trigger_for_status("working"), "working");
        assert_eq!(mood_trigger_for_status("thinking"), "working");
        assert_eq!(mood_trigger_for_status("waiting"), "reminder");
    }

    #[test]
    fn mood_trigger_for_terminal_statuses_uses_canonical_sprite_names() {
        assert_eq!(mood_trigger_for_status("happy"), "happy");
        assert_eq!(mood_trigger_for_status("done"), "happy");
        assert_eq!(mood_trigger_for_status("idle"), "idle");
        assert_eq!(mood_trigger_for_status("unknown"), "idle");
    }

    #[test]
    fn stale_active_state_times_out_working_status() {
        let state = BridgeState {
            status: Some("working".to_string()),
            session_title: Some("Stuck task".to_string()),
            started_at: Some(10),
            sessions: Some(vec![]),
            completed_sessions: Some(vec![]),
            updated_at: Some(100),
        };

        assert!(is_stale_active_state(&state, 130));
        assert!(!is_stale_active_state(&state, 129));
    }

    #[test]
    fn stale_active_state_times_out_waiting_status() {
        let state = BridgeState {
            status: Some("waiting".to_string()),
            session_title: Some("Needs input".to_string()),
            started_at: Some(10),
            sessions: Some(vec![]),
            completed_sessions: Some(vec![]),
            updated_at: Some(200),
        };

        assert!(is_stale_active_state(&state, 230));
    }

    #[test]
    fn stale_active_state_ignores_idle_and_missing_timestamps() {
        let idle_state = BridgeState {
            status: Some("idle".to_string()),
            session_title: None,
            started_at: None,
            sessions: Some(vec![]),
            completed_sessions: Some(vec![]),
            updated_at: Some(100),
        };
        let missing_timestamp = BridgeState {
            status: Some("working".to_string()),
            session_title: None,
            started_at: None,
            sessions: Some(vec![]),
            completed_sessions: Some(vec![]),
            updated_at: None,
        };

        assert!(!is_stale_active_state(&idle_state, 10_000));
        assert!(!is_stale_active_state(&missing_timestamp, 10_000));
    }
}

fn update_badge(state: &BridgeState) -> Result<(), extism_pdk::Error> {
    let badges = badge_items_from_state(state);
    peekoo::badge::set(&badges)?;
    Ok(())
}
