//! Conversation session loading — thin wrapper over pi's built-in session persistence.
//!
//! Uses [`SessionIndex`] to locate the most recent session file for the active
//! workspace and [`Session`] to parse it into frontend-ready DTOs.

use std::path::Path;

use peekoo_agent::{Session, SessionIndex};
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
    /// Chronological list of user/assistant messages.
    pub messages: Vec<SessionMessageDto>,
}

// ────────────────────────────────────────────────────────────────────────────
// Public helpers
// ────────────────────────────────────────────────────────────────────────────

/// Load the most recent session from disk and return its messages.
///
/// Returns `Ok(None)` when there are no prior sessions.
pub async fn load_last_session(
    session_dir: &Path,
    workspace_dir: &Path,
) -> Result<Option<LastSessionDto>, String> {
    let index = SessionIndex::for_sessions_root(session_dir);
    let workspace_cwd = workspace_dir.to_string_lossy().to_string();
    let metas = index
        .list_sessions(Some(&workspace_cwd))
        .map_err(|e| format!("List sessions error: {e}"))?;

    let meta = match metas.first() {
        Some(m) => m,
        None => return Ok(None),
    };

    let session = Session::open(&meta.path)
        .await
        .map_err(|e| format!("Open session error: {e}"))?;

    let dto_messages = session_messages_to_dtos(&session);

    if dto_messages.is_empty() {
        return Ok(None);
    }

    Ok(Some(LastSessionDto {
        session_path: meta.path.clone(),
        messages: dto_messages,
    }))
}

/// Convert in-memory agent messages (serialised as JSON values) to DTOs.
pub fn json_messages_to_dtos(values: &[serde_json::Value]) -> Vec<SessionMessageDto> {
    values.iter().filter_map(json_value_to_dto).collect()
}

// ────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ────────────────────────────────────────────────────────────────────────────

/// Convert pi `Session` messages into DTOs.
///
/// The session's `to_messages_for_current_path()` returns opaque pi `Message`
/// values. We serialise each to JSON and then extract the fields we need so
/// that the conversion logic stays in one place ([`json_value_to_dto`]).
fn session_messages_to_dtos(session: &Session) -> Vec<SessionMessageDto> {
    let messages = session.to_messages_for_current_path();
    messages
        .iter()
        .filter_map(|m| {
            let value = serde_json::to_value(m).ok()?;
            json_value_to_dto(&value)
        })
        .collect()
}

/// Extract a DTO from a single serialised `Message` value.
///
/// The pi `Message` enum serialises with `#[serde(tag = "role", rename_all = "camelCase")]`:
/// - `{ "role": "user", "content": "<text>" | [...blocks], ... }`
/// - `{ "role": "assistant", "content": [...blocks], ... }`
/// - `{ "role": "toolResult", ... }` — skipped (internal plumbing)
/// - `{ "role": "custom", ... }` — skipped
fn json_value_to_dto(value: &serde_json::Value) -> Option<SessionMessageDto> {
    let role = value.get("role")?.as_str()?;

    match role {
        "user" => {
            let text = extract_user_text(value)?;
            Some(SessionMessageDto {
                role: "user".into(),
                text,
            })
        }
        "assistant" => {
            let text = extract_content_block_text(value)?;
            Some(SessionMessageDto {
                role: "assistant".into(),
                text,
            })
        }
        _ => None, // toolResult, custom, etc.
    }
}

/// Extract text from a user message.
///
/// `UserContent` serialises as either:
/// - a plain string (`"content": "hello"`) via `#[serde(untagged)]` `Text` variant
/// - an array of content blocks via the `Blocks` variant
fn extract_user_text(value: &serde_json::Value) -> Option<String> {
    let content = value.get("content")?;
    if let Some(text) = content.as_str() {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        return Some(trimmed.to_string());
    }
    // Blocks variant
    extract_text_from_blocks(content)
}

/// Extract concatenated text from an assistant message's content blocks.
fn extract_content_block_text(value: &serde_json::Value) -> Option<String> {
    let content = value.get("content")?;
    extract_text_from_blocks(content)
}

/// Concatenate all `Text` blocks from a content array.
fn extract_text_from_blocks(content: &serde_json::Value) -> Option<String> {
    let blocks = content.as_array()?;
    let mut parts = Vec::new();
    for block in blocks {
        if block.get("type").and_then(|t| t.as_str()) == Some("text")
            && let Some(text) = block.get("text").and_then(|t| t.as_str())
            && !text.is_empty()
        {
            parts.push(text.to_string());
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use peekoo_agent::ContentBlock;
    use pi::model::{AssistantMessage, StopReason, TextContent, Usage, UserContent};
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        path.push(format!("peekoo-agent-app-conversation-{prefix}-{nanos}"));
        std::fs::create_dir_all(&path).expect("create temp test dir");
        path
    }

    fn assistant_message(text: &str) -> AssistantMessage {
        AssistantMessage {
            content: vec![ContentBlock::Text(TextContent::new(text))],
            api: "test".into(),
            provider: "test".into(),
            model: "test-model".into(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        }
    }

    async fn write_indexed_session(
        session_root: &Path,
        workspace_dir: &Path,
        file_name: &str,
        user_text: &str,
    ) -> String {
        let mut session = Session::in_memory();
        session.header.cwd = workspace_dir.to_string_lossy().to_string();
        session.path = Some(session_root.join(file_name));
        session.append_message(pi::session::SessionMessage::User {
            content: UserContent::Text(user_text.to_string()),
            timestamp: Some(0),
        });
        session.append_message(pi::session::SessionMessage::Assistant {
            message: assistant_message("assistant reply"),
        });
        session.save().await.expect("save session");

        let path = session
            .path
            .as_ref()
            .expect("session path")
            .display()
            .to_string();
        SessionIndex::for_sessions_root(session_root)
            .reindex_all()
            .expect("reindex sessions");
        path
    }

    #[test]
    fn user_text_message_to_dto() {
        let value = json!({
            "role": "user",
            "content": "Hello, world!",
            "timestamp": 1700000000
        });
        let dto = json_value_to_dto(&value).expect("should produce dto");
        assert_eq!(dto.role, "user");
        assert_eq!(dto.text, "Hello, world!");
    }

    #[test]
    fn assistant_message_to_dto() {
        let value = json!({
            "role": "assistant",
            "content": [
                { "type": "text", "text": "Hi there!" }
            ],
            "api": "anthropic",
            "provider": "anthropic",
            "model": "claude-sonnet-4",
            "usage": { "input": 10, "output": 5 },
            "stopReason": "stop",
            "timestamp": 1700000000
        });
        let dto = json_value_to_dto(&value).expect("should produce dto");
        assert_eq!(dto.role, "assistant");
        assert_eq!(dto.text, "Hi there!");
    }

    #[test]
    fn assistant_multiple_text_blocks_concatenated() {
        let value = json!({
            "role": "assistant",
            "content": [
                { "type": "thinking", "thinking": "hmm..." },
                { "type": "text", "text": "Part one " },
                { "type": "text", "text": "part two." }
            ],
            "api": "test",
            "provider": "test",
            "model": "test",
            "usage": {},
            "stopReason": "stop",
            "timestamp": 0
        });
        let dto = json_value_to_dto(&value).expect("should produce dto");
        assert_eq!(dto.text, "Part one part two.");
    }

    #[test]
    fn assistant_multiple_text_blocks_preserve_newlines_and_spacing() {
        let value = json!({
            "role": "assistant",
            "content": [
                { "type": "text", "text": "```rust\nfn main() {\n" },
                { "type": "text", "text": "    println!(\"hi\");\n}\n```" }
            ],
            "api": "test",
            "provider": "test",
            "model": "test",
            "usage": {},
            "stopReason": "stop",
            "timestamp": 0
        });
        let dto = json_value_to_dto(&value).expect("should produce dto");
        assert_eq!(
            dto.text,
            "```rust\nfn main() {\n    println!(\"hi\");\n}\n```"
        );
    }

    #[test]
    fn tool_result_is_skipped() {
        let value = json!({
            "role": "toolResult",
            "toolCallId": "tc_1",
            "toolName": "read",
            "content": [],
            "isError": false,
            "timestamp": 0
        });
        assert!(json_value_to_dto(&value).is_none());
    }

    #[test]
    fn custom_message_is_skipped() {
        let value = json!({
            "role": "custom",
            "content": "internal",
            "customType": "test",
            "display": false,
            "timestamp": 0
        });
        assert!(json_value_to_dto(&value).is_none());
    }

    #[test]
    fn empty_assistant_text_is_skipped() {
        let value = json!({
            "role": "assistant",
            "content": [
                { "type": "thinking", "thinking": "only thinking" }
            ],
            "api": "test",
            "provider": "test",
            "model": "test",
            "usage": {},
            "stopReason": "stop",
            "timestamp": 0
        });
        assert!(json_value_to_dto(&value).is_none());
    }

    #[test]
    fn json_messages_to_dtos_filters_correctly() {
        let values = vec![
            json!({ "role": "user", "content": "hello", "timestamp": 0 }),
            json!({ "role": "toolResult", "toolCallId": "t1", "toolName": "r", "content": [], "isError": false, "timestamp": 0 }),
            json!({ "role": "assistant", "content": [{ "type": "text", "text": "world" }], "api": "t", "provider": "t", "model": "t", "usage": {}, "stopReason": "stop", "timestamp": 0 }),
        ];
        let dtos = json_messages_to_dtos(&values);
        assert_eq!(dtos.len(), 2);
        assert_eq!(dtos[0].role, "user");
        assert_eq!(dtos[0].text, "hello");
        assert_eq!(dtos[1].role, "assistant");
        assert_eq!(dtos[1].text, "world");
    }

    #[test]
    fn user_blocks_content_extracts_text() {
        let value = json!({
            "role": "user",
            "content": [
                { "type": "text", "text": "Block text" }
            ],
            "timestamp": 0
        });
        let dto = json_value_to_dto(&value).expect("should produce dto");
        assert_eq!(dto.role, "user");
        assert_eq!(dto.text, "Block text");
    }

    #[tokio::test]
    async fn load_last_session_filters_to_workspace() {
        let session_root = temp_test_dir("session-root");
        let workspace_a = temp_test_dir("workspace-a");
        let workspace_b = temp_test_dir("workspace-b");

        let expected_path =
            write_indexed_session(&session_root, &workspace_a, "workspace-a.jsonl", "hello a")
                .await;
        let _other_path =
            write_indexed_session(&session_root, &workspace_b, "workspace-b.jsonl", "hello b")
                .await;

        let dto = load_last_session(&session_root, &workspace_a)
            .await
            .expect("load last session")
            .expect("workspace session");

        assert_eq!(dto.session_path, expected_path);
        assert_eq!(dto.messages.len(), 2);
        assert_eq!(dto.messages[0].text, "hello a");
    }
}
