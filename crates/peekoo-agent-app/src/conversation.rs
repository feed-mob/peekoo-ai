//! Conversation session loading — thin wrapper over SQLite session persistence.
//!
//! Uses the new SessionStore to locate and load conversation history.

use peekoo_agent::backend::{ContentBlock, Message, MessageRole};
use peekoo_agent::session_store::SessionStore;
use serde::Serialize;
use std::path::Path;

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

/// Find the most recent non-empty session in the shared application database.
pub fn find_last_session() -> anyhow::Result<Option<LastSessionDto>> {
    let db_path = peekoo_paths::peekoo_settings_db_path()
        .map_err(|e| anyhow::anyhow!("Failed to locate settings db: {e}"))?;
    find_last_session_from_db(&db_path)
}

/// Convert a raw session into frontend-ready DTOs.
///
/// Filters out system/tool messages and flattens complex content blocks
/// into plain text suitable for the chat panel.
pub fn json_messages_to_dtos(messages: &[String]) -> Vec<SessionMessageDto> {
    messages
        .iter()
        .filter_map(|raw| serde_json::from_str::<Message>(raw).ok())
        .filter_map(message_to_dto)
        .collect()
}

fn find_last_session_from_db(db_path: &Path) -> anyhow::Result<Option<LastSessionDto>> {
    let store = SessionStore::open(&db_path.to_path_buf())?;

    for session in store.list_sessions(Some("active"))? {
        let messages = store.load_messages(&session.id)?;
        if messages.is_empty() {
            continue;
        }

        let message_json: Vec<String> = messages
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<_, _>>()?;
        let dtos = json_messages_to_dtos(&message_json);
        if dtos.is_empty() {
            continue;
        }

        return Ok(Some(LastSessionDto {
            session_path: session.id,
            last_message_timestamp: duration_debug_to_millis(&session.updated_at),
            messages: dtos,
        }));
    }

    Ok(None)
}

fn message_to_dto(message: Message) -> Option<SessionMessageDto> {
    match message.role {
        MessageRole::User | MessageRole::Assistant => {
            let text = flatten_content_blocks(&message.content);
            if text.is_empty() {
                None
            } else {
                Some(SessionMessageDto {
                    role: match message.role {
                        MessageRole::User => "user".to_string(),
                        MessageRole::Assistant => "assistant".to_string(),
                        _ => unreachable!(),
                    },
                    text,
                })
            }
        }
        MessageRole::System | MessageRole::Tool => None,
    }
}

fn flatten_content_blocks(blocks: &[ContentBlock]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.trim().to_string()),
            ContentBlock::ToolResult { content, .. } => Some(content.trim().to_string()),
            ContentBlock::Thinking { .. }
            | ContentBlock::ToolUse { .. }
            | ContentBlock::Image { .. } => None,
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn duration_debug_to_millis(value: &str) -> i64 {
    let trimmed = value.trim();
    let numeric = trimmed.strip_suffix('s').unwrap_or(trimmed);
    numeric
        .parse::<f64>()
        .map(|seconds| (seconds * 1000.0) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use peekoo_agent::backend::{ContentBlock, Message, MessageRole};
    use peekoo_agent::session_store::SessionStore;
    use tempfile::NamedTempFile;

    #[test]
    fn finds_latest_non_empty_session_from_db() {
        let db = NamedTempFile::new().expect("temp db");
        let store = SessionStore::open(&db.path().to_path_buf()).expect("open store");

        let old_session = store
            .create_session(
                Some("Old"),
                "pi-acp",
                "npx",
                &["pi-acp".to_string()],
                &std::env::temp_dir(),
                None,
                &[],
            )
            .expect("create old session");
        store
            .append_message(
                &old_session,
                &Message {
                    role: MessageRole::User,
                    content: vec![ContentBlock::Text {
                        text: "hello".to_string(),
                    }],
                    tool_calls: None,
                    tool_call_id: None,
                },
                Some("pi-acp"),
                Some("claude-sonnet-4-6"),
                None,
            )
            .expect("append old message");

        let new_empty_session = store
            .create_session(
                Some("New Empty"),
                "opencode",
                "npx",
                &["opencode-ai".to_string()],
                &std::env::temp_dir(),
                None,
                &[],
            )
            .expect("create empty session");
        assert_eq!(
            store.get_message_count(&new_empty_session).expect("count"),
            0
        );

        let dto = find_last_session_from_db(db.path()).expect("load last session");
        let dto = dto.expect("expected a session dto");

        assert_eq!(dto.messages.len(), 1);
        assert_eq!(dto.messages[0].role, "user");
        assert_eq!(dto.messages[0].text, "hello");
        assert_eq!(dto.session_path, old_session);
    }

    #[test]
    fn flattens_text_and_tool_result_blocks() {
        let messages = vec![
            serde_json::to_string(&Message {
                role: MessageRole::Assistant,
                content: vec![
                    ContentBlock::Thinking {
                        thinking: "hidden".to_string(),
                    },
                    ContentBlock::Text {
                        text: "Answer".to_string(),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id: "tool-1".to_string(),
                        content: "done".to_string(),
                        is_error: false,
                    },
                ],
                tool_calls: None,
                tool_call_id: None,
            })
            .expect("serialize"),
        ];

        let dtos = json_messages_to_dtos(&messages);

        assert_eq!(dtos.len(), 1);
        assert_eq!(dtos[0].role, "assistant");
        assert_eq!(dtos[0].text, "Answer\ndone");
    }
}
