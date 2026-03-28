//! Conversation session loading — thin wrapper over SQLite session persistence.
//!
//! Uses the new SessionStore to locate and load conversation history.
//! TODO: Reimplement using peekoo_agent::session_store after migration

use serde::Serialize;

// ────────────────────────────────────────────────────────────────────────────
// DTOs
// ────────────────────────────────────────────────────────────────────────────

/// A single message suitable for the chat panel frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMessageDto {
    /// `"user"` or `"assistant"`.
    pub role: String,
    /// Plain-text content extracted from the message's content blocks.
    pub text: String,
}

/// The payload returned by `chat_get_last_session`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastSessionDto {
    /// Path to the session file on disk (used to resume the session).
    pub session_path: String,
    /// Unix timestamp (milliseconds) of the most recent message.
    pub last_message_timestamp: i64,
    /// All messages in chronological order.
    pub messages: Vec<SessionMessageDto>,
}

// ────────────────────────────────────────────────────────────────────────────
// Functions
// ────────────────────────────────────────────────────────────────────────────

/// Find the most recent non-empty session in the current workspace.
pub fn find_last_session(_cwd: &std::path::Path) -> anyhow::Result<Option<LastSessionDto>> {
    // TODO: Reimplement using SessionStore
    // For now, return None to indicate no sessions found
    Ok(None)
}

/// Convert a raw session into frontend-ready DTOs.
///
/// Filters out system/tool messages and flattens complex content blocks
/// into plain text suitable for the chat panel.
pub fn json_messages_to_dtos(_messages: &[String]) -> Vec<SessionMessageDto> {
    // TODO: Reimplement using new Message types
    // For now, return empty vector
    vec![]
}
