//! Session store for agent conversation persistence in SQLite
//!
//! This module provides storage and retrieval of agent sessions and their
//! conversation history, supporting peekoo-managed persistence while
//! allowing agent-specific state to be stored opaquely.

use crate::backend::{ContentBlock, Message, MessageRole, TokenUsage};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};

/// Session metadata for listing
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub title: Option<String>,
    pub provider: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Full session data
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub title: Option<String>,
    pub status: String,
    pub current_provider: String,
    pub provider_command: String,
    pub provider_args: Vec<String>,
    pub working_directory: PathBuf,
    pub system_prompt: Option<String>,
    pub skills: Vec<String>,
    pub provider_state: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// Session store for agent conversation persistence
pub struct SessionStore {
    conn: Connection,
    db_path: PathBuf,
}

impl SessionStore {
    /// Open or create session store at the given path
    pub fn open(db_path: &PathBuf) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;

        // Run migrations to ensure tables exist
        peekoo_persistence_sqlite::run_all_migrations(&conn)
            .map_err(|e| anyhow::anyhow!("Failed to run migrations: {e}"))?;

        Ok(Self {
            conn,
            db_path: db_path.clone(),
        })
    }

    /// Create an in-memory store (for testing)
    #[cfg(test)]
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        // For in-memory testing, run all migrations
        peekoo_persistence_sqlite::run_all_migrations(&conn)
            .map_err(|e| anyhow::anyhow!("Failed to run migrations: {e}"))?;
        Ok(Self {
            conn,
            db_path: PathBuf::from(":memory:"),
        })
    }

    /// Get the database file path
    pub fn db_path(&self) -> PathBuf {
        self.db_path.clone()
    }

    /// Get a reference to the connection for testing purposes
    #[cfg(test)]
    pub fn test_conn(&self) -> &Connection {
        &self.conn
    }

    /// Create a new session
    #[allow(clippy::too_many_arguments)]
    pub fn create_session(
        &self,
        title: Option<&str>,
        provider: &str,
        command: &str,
        args: &[String],
        working_dir: &Path,
        system_prompt: Option<&str>,
        skills: &[String],
    ) -> anyhow::Result<String> {
        let session_id = generate_session_id();
        let now = now_iso8601();

        self.conn.execute(
            "INSERT INTO agent_sessions (
                id, title, status, current_provider, provider_command, 
                provider_args_json, working_directory, system_prompt, 
                skills_json, runtime_id, llm_provider_id, model_id,
                created_at, updated_at, last_activity_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?4, NULL, NULL, ?10, ?10, ?10)",
            params![
                &session_id,
                title,
                "active",
                provider,
                command,
                &serde_json::to_string(args)?,
                working_dir.to_str(),
                system_prompt,
                &serde_json::to_string(skills)?,
                &now,
            ],
        )?;

        Ok(session_id)
    }

    /// Load session metadata
    pub fn load_session(&self, session_id: &str) -> anyhow::Result<Option<Session>> {
        let session = self
            .conn
            .query_row(
                "SELECT 
                id, title, status, current_provider, provider_command,
                provider_args_json, working_directory, system_prompt,
                skills_json, provider_state_json, created_at, updated_at
            FROM agent_sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    let provider_state_json: Option<String> = row.get(9)?;
                    let provider_state =
                        provider_state_json.and_then(|s| serde_json::from_str(&s).ok());

                    let args_json: String = row.get(5)?;
                    let skills_json: String = row.get(8)?;

                    Ok(Session {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        status: row.get(2)?,
                        current_provider: row.get(3)?,
                        provider_command: row.get(4)?,
                        provider_args: serde_json::from_str(&args_json).unwrap_or_default(),
                        working_directory: PathBuf::from(row.get::<_, String>(6)?),
                        system_prompt: row.get(7)?,
                        skills: serde_json::from_str(&skills_json).unwrap_or_default(),
                        provider_state,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                    })
                },
            )
            .optional()?;

        Ok(session)
    }

    /// Load conversation history
    pub fn load_messages(&self, session_id: &str) -> anyhow::Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT role, content_type, content_json, tool_name, tool_call_id, provider
            FROM session_messages 
            WHERE session_id = ?1 
            ORDER BY sequence_num ASC",
        )?;

        let messages: Vec<_> = stmt
            .query_map(params![session_id], |row| {
                let role_str: String = row.get(0)?;
                let role = match role_str.as_str() {
                    "system" => MessageRole::System,
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    "tool" => MessageRole::Tool,
                    _ => MessageRole::User,
                };

                let content_json: String = row.get(2)?;
                let content: Vec<ContentBlock> =
                    serde_json::from_str(&content_json).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                Ok(Message {
                    role,
                    content,
                    tool_calls: None,
                    tool_call_id: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    /// Append a message to the session
    pub fn append_message(
        &self,
        session_id: &str,
        message: &Message,
        provider: Option<&str>,
        model: Option<&str>,
        usage: Option<&TokenUsage>,
    ) -> anyhow::Result<()> {
        let now = now_iso8601();

        // Get next sequence number
        let seq_num: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(sequence_num), 0) + 1 FROM session_messages WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;

        let role_str = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };

        let content_type = if message.content.is_empty() {
            "text"
        } else {
            match &message.content[0] {
                ContentBlock::Text { .. } => "text",
                ContentBlock::Thinking { .. } => "thinking",
                ContentBlock::ToolUse { .. } => "tool_use",
                ContentBlock::ToolResult { .. } => "tool_result",
                ContentBlock::Image { .. } => "image",
            }
        };

        let content_json = serde_json::to_string(&message.content)?;

        // Extract tool name if present
        let tool_name = message.content.iter().find_map(|block| {
            if let ContentBlock::ToolUse { name, .. } = block {
                Some(name.clone())
            } else {
                None
            }
        });

        self.conn.execute(
            "INSERT INTO session_messages (
                id, session_id, sequence_num, role, content_type,
                content_json, tool_name, tool_call_id, provider, model,
                input_tokens, output_tokens, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                generate_message_id(),
                session_id,
                seq_num,
                role_str,
                content_type,
                content_json,
                tool_name,
                message.tool_call_id.as_deref(),
                provider,
                model,
                usage.map(|u| u.input_tokens as i64),
                usage.map(|u| u.output_tokens as i64),
                &now,
            ],
        )?;

        // Update session last_activity_at
        self.conn.execute(
            "UPDATE agent_sessions SET updated_at = ?1, last_activity_at = ?1 WHERE id = ?2",
            params![&now, session_id],
        )?;

        Ok(())
    }

    /// Update provider state (for resuming sessions)
    pub fn update_provider_state(
        &self,
        session_id: &str,
        state: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let now = now_iso8601();

        self.conn.execute(
            "UPDATE agent_sessions SET provider_state_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![state.to_string(), &now, session_id,],
        )?;

        Ok(())
    }

    /// Update session status (active, paused, closed)
    pub fn update_session_status(&self, session_id: &str, status: &str) -> anyhow::Result<()> {
        let now = now_iso8601();
        let closed_at = if status == "closed" { Some(&now) } else { None };

        self.conn.execute(
            "UPDATE agent_sessions SET status = ?1, updated_at = ?2, closed_at = ?3 WHERE id = ?4",
            params![status, &now, closed_at, session_id],
        )?;

        Ok(())
    }

    /// Update runtime-scoped provider/model context for a session.
    pub fn update_runtime_context(
        &self,
        session_id: &str,
        llm_provider_id: Option<&str>,
        model_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let now = now_iso8601();

        self.conn.execute(
            "UPDATE agent_sessions
             SET llm_provider_id = ?1, model_id = ?2, updated_at = ?3
             WHERE id = ?4",
            params![llm_provider_id, model_id, &now, session_id],
        )?;

        Ok(())
    }

    /// Switch provider mid-session
    pub fn switch_provider(
        &self,
        session_id: &str,
        new_provider: &str,
        new_command: &str,
        new_args: &[String],
    ) -> anyhow::Result<()> {
        let now = now_iso8601();

        self.conn.execute(
            "UPDATE agent_sessions SET 
                current_provider = ?1,
                provider_command = ?2,
                provider_args_json = ?3,
                provider_state_json = NULL,
                updated_at = ?4
            WHERE id = ?5",
            params![
                new_provider,
                new_command,
                &serde_json::to_string(new_args)?,
                &now,
                session_id,
            ],
        )?;

        // Add a system message noting the provider switch
        let switch_message = Message {
            role: MessageRole::System,
            content: vec![ContentBlock::Text {
                text: format!("Provider switched to: {}", new_provider),
            }],
            tool_calls: None,
            tool_call_id: None,
        };

        self.append_message(session_id, &switch_message, Some(new_provider), None, None)?;

        Ok(())
    }

    /// List all sessions (with optional status filter)
    pub fn list_sessions(
        &self,
        status_filter: Option<&str>,
    ) -> anyhow::Result<Vec<SessionSummary>> {
        if let Some(status) = status_filter {
            let mut stmt = self.conn.prepare(
                "SELECT id, title, current_provider, status, created_at, updated_at 
                 FROM agent_sessions WHERE status = ?1 ORDER BY updated_at DESC",
            )?;

            let sessions: Vec<_> = stmt
                .query_map(params![status], |row| {
                    Ok(SessionSummary {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        provider: row.get(2)?,
                        status: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(sessions)
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, title, current_provider, status, created_at, updated_at 
                 FROM agent_sessions ORDER BY updated_at DESC",
            )?;

            let sessions: Vec<_> = stmt
                .query_map([], |row| {
                    Ok(SessionSummary {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        provider: row.get(2)?,
                        status: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(sessions)
        }
    }

    /// Delete a session and all its messages
    pub fn delete_session(&self, session_id: &str) -> anyhow::Result<()> {
        // Due to ON DELETE CASCADE, deleting the session will delete all messages
        self.conn.execute(
            "DELETE FROM agent_sessions WHERE id = ?1",
            params![session_id],
        )?;

        Ok(())
    }

    /// Get message count for a session
    pub fn get_message_count(&self, session_id: &str) -> anyhow::Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM session_messages WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }
}

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("sess_{}_{}", timestamp, generate_random_suffix())
}

fn generate_message_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros();
    format!("msg_{}", timestamp)
}

fn generate_random_suffix() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect::<String>()
        .to_lowercase()
}

fn now_iso8601() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now();
    let datetime = std::time::UNIX_EPOCH + now.duration_since(std::time::UNIX_EPOCH).unwrap();
    // Simple ISO8601 format
    format!("{:?}", datetime)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> SessionStore {
        SessionStore::open_in_memory().expect("Failed to create test store")
    }

    #[test]
    fn test_create_session() {
        let store = create_test_store();
        let session_id = store
            .create_session(
                Some("Test Session"),
                "pi-acp",
                "npx",
                &["pi-acp".to_string()],
                &PathBuf::from("/tmp/test"),
                Some("You are helpful."),
                &["skill1".to_string(), "skill2".to_string()],
            )
            .unwrap();

        assert!(!session_id.is_empty());
        assert!(session_id.starts_with("sess_"));

        let (runtime_id, llm_provider_id, model_id): (
            Option<String>,
            Option<String>,
            Option<String>,
        ) = store
            .test_conn()
            .query_row(
                "SELECT runtime_id, llm_provider_id, model_id FROM agent_sessions WHERE id = ?1",
                params![&session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(runtime_id.as_deref(), Some("pi-acp"));
        assert!(llm_provider_id.is_none());
        assert!(model_id.is_none());
    }

    #[test]
    fn test_update_runtime_context() {
        let store = create_test_store();
        let session_id = store
            .create_session(
                Some("Test Session"),
                "pi-acp",
                "npx",
                &["pi-acp".to_string()],
                &PathBuf::from("/tmp/test"),
                None,
                &[],
            )
            .unwrap();

        store
            .update_runtime_context(&session_id, Some("anthropic"), Some("claude-sonnet-4-6"))
            .unwrap();

        let (provider_id, model_id): (Option<String>, Option<String>) = store
            .test_conn()
            .query_row(
                "SELECT llm_provider_id, model_id FROM agent_sessions WHERE id = ?1",
                params![&session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(provider_id.as_deref(), Some("anthropic"));
        assert_eq!(model_id.as_deref(), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn test_load_session() {
        let store = create_test_store();
        let session_id = store
            .create_session(
                Some("Test Session"),
                "pi-acp",
                "npx",
                &["pi-acp".to_string()],
                &PathBuf::from("/tmp/test"),
                Some("You are helpful."),
                &["skill1".to_string()],
            )
            .unwrap();

        let session = store.load_session(&session_id).unwrap();
        assert!(session.is_some());

        let session = session.unwrap();
        assert_eq!(session.title, Some("Test Session".to_string()));
        assert_eq!(session.current_provider, "pi-acp");
        assert_eq!(session.provider_command, "npx");
        assert_eq!(session.provider_args, vec!["pi-acp"]);
        assert_eq!(session.working_directory, PathBuf::from("/tmp/test"));
        assert_eq!(session.system_prompt, Some("You are helpful.".to_string()));
        assert_eq!(session.skills, vec!["skill1"]);
    }

    #[test]
    fn test_load_nonexistent_session() {
        let store = create_test_store();
        let session = store.load_session("nonexistent").unwrap();
        assert!(session.is_none());
    }

    #[test]
    fn test_append_and_load_messages() {
        let store = create_test_store();
        let session_id = store
            .create_session(
                None,
                "pi-acp",
                "npx",
                &[],
                &PathBuf::from("/tmp"),
                None,
                &[],
            )
            .unwrap();

        // Append a user message
        let user_msg = Message {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
            tool_calls: None,
            tool_call_id: None,
        };
        store
            .append_message(&session_id, &user_msg, Some("pi-acp"), None, None)
            .unwrap();

        // Append an assistant message
        let assistant_msg = Message {
            role: MessageRole::Assistant,
            content: vec![ContentBlock::Text {
                text: "Hi!".to_string(),
            }],
            tool_calls: None,
            tool_call_id: None,
        };
        store
            .append_message(
                &session_id,
                &assistant_msg,
                Some("pi-acp"),
                Some("claude-3.5"),
                Some(&TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                }),
            )
            .unwrap();

        // Load messages
        let messages = store.load_messages(&session_id).unwrap();
        assert_eq!(messages.len(), 2);
        assert!(matches!(messages[0].role, MessageRole::User));
        assert!(matches!(messages[1].role, MessageRole::Assistant));
    }

    #[test]
    fn test_update_provider_state() {
        let store = create_test_store();
        let session_id = store
            .create_session(
                None,
                "pi-acp",
                "npx",
                &[],
                &PathBuf::from("/tmp"),
                None,
                &[],
            )
            .unwrap();

        let state = serde_json::json!({ "session_id": "abc123", "context": "test context" });
        store.update_provider_state(&session_id, &state).unwrap();

        let session = store.load_session(&session_id).unwrap().unwrap();
        assert_eq!(session.provider_state, Some(state));
    }

    #[test]
    fn test_update_session_status() {
        let store = create_test_store();
        let session_id = store
            .create_session(
                None,
                "pi-acp",
                "npx",
                &[],
                &PathBuf::from("/tmp"),
                None,
                &[],
            )
            .unwrap();

        store.update_session_status(&session_id, "closed").unwrap();

        let session = store.load_session(&session_id).unwrap().unwrap();
        assert_eq!(session.status, "closed");
    }

    #[test]
    fn test_switch_provider() {
        let store = create_test_store();
        let session_id = store
            .create_session(
                None,
                "pi-acp",
                "npx",
                &["pi-acp".to_string()],
                &PathBuf::from("/tmp"),
                None,
                &[],
            )
            .unwrap();

        // Switch provider
        store
            .switch_provider(&session_id, "opencode", "opencode", &["acp".to_string()])
            .unwrap();

        // Check session updated
        let session = store.load_session(&session_id).unwrap().unwrap();
        assert_eq!(session.current_provider, "opencode");
        assert_eq!(session.provider_command, "opencode");
        assert_eq!(session.provider_args, vec!["acp"]);
        assert!(session.provider_state.is_none()); // Should be reset

        // Check system message was added
        let messages = store.load_messages(&session_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(matches!(messages[0].role, MessageRole::System));
    }

    #[test]
    fn test_list_sessions() {
        let store = create_test_store();

        // Create multiple sessions
        let id1 = store
            .create_session(
                Some("Session 1"),
                "pi-acp",
                "npx",
                &[],
                &PathBuf::from("/tmp"),
                None,
                &[],
            )
            .unwrap();
        let id2 = store
            .create_session(
                Some("Session 2"),
                "opencode",
                "npx",
                &[],
                &PathBuf::from("/tmp"),
                None,
                &[],
            )
            .unwrap();

        // Close one session
        store.update_session_status(&id2, "closed").unwrap();

        // List all sessions
        let all_sessions = store.list_sessions(None).unwrap();
        assert_eq!(all_sessions.len(), 2);

        // List only active sessions
        let active_sessions = store.list_sessions(Some("active")).unwrap();
        assert_eq!(active_sessions.len(), 1);
        assert_eq!(active_sessions[0].id, id1);
    }

    #[test]
    fn test_delete_session() {
        let store = create_test_store();
        let session_id = store
            .create_session(
                None,
                "pi-acp",
                "npx",
                &[],
                &PathBuf::from("/tmp"),
                None,
                &[],
            )
            .unwrap();

        // Add a message
        let msg = Message {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: "Test".to_string(),
            }],
            tool_calls: None,
            tool_call_id: None,
        };
        store
            .append_message(&session_id, &msg, None, None, None)
            .unwrap();

        // Delete session
        store.delete_session(&session_id).unwrap();

        // Session should be gone
        assert!(store.load_session(&session_id).unwrap().is_none());

        // Messages should also be gone (cascade delete)
        let messages = store.load_messages(&session_id).unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_get_message_count() {
        let store = create_test_store();
        let session_id = store
            .create_session(
                None,
                "pi-acp",
                "npx",
                &[],
                &PathBuf::from("/tmp"),
                None,
                &[],
            )
            .unwrap();

        assert_eq!(store.get_message_count(&session_id).unwrap(), 0);

        // Add messages
        for i in 0..5 {
            let msg = Message {
                role: MessageRole::User,
                content: vec![ContentBlock::Text {
                    text: format!("Message {}", i),
                }],
                tool_calls: None,
                tool_call_id: None,
            };
            store
                .append_message(&session_id, &msg, None, None, None)
                .unwrap();
        }

        assert_eq!(store.get_message_count(&session_id).unwrap(), 5);
    }

    #[test]
    fn test_message_with_tool_use() {
        let store = create_test_store();
        let session_id = store
            .create_session(
                None,
                "pi-acp",
                "npx",
                &[],
                &PathBuf::from("/tmp"),
                None,
                &[],
            )
            .unwrap();

        let msg = Message {
            role: MessageRole::Assistant,
            content: vec![ContentBlock::ToolUse {
                id: "call_123".to_string(),
                name: "read_file".to_string(),
                input: serde_json::json!({ "path": "/tmp/test.txt" }),
            }],
            tool_calls: None,
            tool_call_id: None,
        };
        store
            .append_message(&session_id, &msg, Some("pi-acp"), Some("claude-3.5"), None)
            .unwrap();

        let messages = store.load_messages(&session_id).unwrap();
        assert_eq!(messages.len(), 1);

        match &messages[0].content[0] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_123");
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "/tmp/test.txt");
            }
            _ => panic!("Expected ToolUse content block"),
        }
    }
}
