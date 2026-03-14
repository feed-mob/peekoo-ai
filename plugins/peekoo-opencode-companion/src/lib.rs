#![cfg_attr(target_arch = "wasm32", no_main)]

use peekoo_plugin_sdk::prelude::*;

// ── Bridge file schema ─────────────────────────────────────────

#[derive(Deserialize, Default)]
struct BridgeState {
    status: Option<String>,
    session_title: Option<String>,
    started_at: Option<u64>,
    #[allow(dead_code)]
    updated_at: Option<u64>,
}

// ── Constants ──────────────────────────────────────────────────

const SCHEDULE_KEY: &str = "poll-opencode";
const POLL_INTERVAL_SECS: u64 = 2;
const STATE_LAST_STATUS: &str = "last_status";
const MAX_DISPLAY_LEN: usize = 30;

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

    let current_status = state.status.as_deref().unwrap_or("idle");
    let previous_status: String =
        peekoo::state::get(STATE_LAST_STATUS)?.unwrap_or_else(|| "idle".to_string());

    // Only act on status changes
    if current_status != previous_status {
        handle_status_change(current_status, &state)?;
        peekoo::state::set(STATE_LAST_STATUS, &current_status.to_string())?;
    }

    // Always update badge when active (elapsed time changes)
    if current_status == "working" || current_status == "thinking" {
        update_badge(&state)?;
    }

    Ok(())
}

fn handle_status_change(new_status: &str, state: &BridgeState) -> Result<(), extism_pdk::Error> {
    match new_status {
        "working" => {
            peekoo::mood::set("opencode-working", true)?;
            update_badge(state)?;
        }
        "thinking" => {
            peekoo::mood::set("opencode-working", true)?;
            update_badge(state)?;
        }
        "happy" | "done" => {
            peekoo::mood::set("opencode-done", false)?;

            let body = match state.session_title.as_deref() {
                Some(t) if !t.is_empty() => {
                    format!("🎉 {} is done!", truncate_title(t))
                }
                _ => "🎉 OpenCode has finished working".to_string(),
            };
            let _ = peekoo::notify::send("OpenCode", &body);

            peekoo::badge::set(&[])?;
        }
        _ => {
            // "idle" or unknown
            peekoo::mood::set("opencode-idle", false)?;
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
}

fn update_badge(state: &BridgeState) -> Result<(), extism_pdk::Error> {
    let title = state.session_title.as_deref().unwrap_or("Working...");

    let display_title = truncate_title(title);

    let elapsed_label = match state.started_at {
        Some(started) => {
            // We don't have access to system time in WASM, so we use
            // countdown_secs to show relative time. The frontend will
            // tick this down. Since we want to show elapsed time (counting
            // up), we don't use countdown_secs and instead format a value.
            let _ = started; // started_at is tracked by the bridge writer
            String::new()
        }
        None => String::new(),
    };

    let value = if elapsed_label.is_empty() {
        display_title
    } else {
        format!("{display_title} ({elapsed_label})")
    };

    peekoo::badge::set(&[BadgeItem {
        label: "OpenCode".into(),
        value,
        icon: Some("activity".into()),
        countdown_secs: None,
    }])?;

    Ok(())
}
