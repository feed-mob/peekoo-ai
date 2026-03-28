//! Task-scoped MCP tool wrappers for the ACP agent.
//!
//! The shared [`peekoo_agent::mcp_client::connect_http_mcp_tools`] adapter
//! connects to the MCP server and returns raw tools. For scheduled task
//! execution, some tools need `task_id` injected automatically so the agent
//! prompt stays simple (no need to pass task_id explicitly).
//!
//! [`TaskScopedTool`] wraps any `pi::tools::Tool` and injects a fixed
//! `task_id` field into the JSON arguments before forwarding the call.

use async_trait::async_trait;
use peekoo_agent::AgentEvent;
use pi::error::Result;
use pi::model::ContentBlock;
use pi::tools::{Tool, ToolOutput, ToolUpdate};

/// Tool names that require automatic `task_id` injection.
const TASK_SCOPED_TOOLS: &[&str] = &["task_comment", "update_task_status", "update_task_labels"];

/// Wraps a `pi::tools::Tool` and injects `task_id` into every call.
///
/// Used by the ACP agent so the LLM can call task tools without needing to
/// know the current task ID — it is injected from the task context.
pub struct TaskScopedTool {
    inner: Box<dyn Tool>,
    task_id: String,
}

impl TaskScopedTool {
    pub fn new(inner: Box<dyn Tool>, task_id: String) -> Self {
        Self { inner, task_id }
    }

    /// Returns true if this tool name should have `task_id` injected.
    pub fn needs_scoping(tool_name: &str) -> bool {
        TASK_SCOPED_TOOLS.contains(&tool_name)
    }
}

#[async_trait]
impl Tool for TaskScopedTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn label(&self) -> &str {
        self.inner.label()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    /// Returns a simplified schema that omits `task_id` — it is injected
    /// automatically, so the LLM does not need to supply it.
    fn parameters(&self) -> serde_json::Value {
        let mut schema = self.inner.parameters();
        // Remove task_id from required and properties so the LLM doesn't see it.
        if let Some(props) = schema.get_mut("properties").and_then(|p| p.as_object_mut()) {
            props.remove("task_id");
        }
        if let Some(required) = schema.get_mut("required").and_then(|r| r.as_array_mut()) {
            required.retain(|v| v.as_str() != Some("task_id"));
        }
        schema
    }

    async fn execute(
        &self,
        tool_call_id: &str,
        mut input: serde_json::Value,
        on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        // Inject task_id before forwarding.
        if let Some(obj) = input.as_object_mut() {
            obj.insert(
                "task_id".to_string(),
                serde_json::Value::String(self.task_id.clone()),
            );
        }
        self.inner.execute(tool_call_id, input, on_update).await
    }

    fn is_read_only(&self) -> bool {
        self.inner.is_read_only()
    }
}

/// Summarize an agent event for streaming back to the ACP client.
pub fn summarize_agent_event(event: &AgentEvent) -> Option<String> {
    match event {
        AgentEvent::ToolExecutionStart {
            tool_name, args, ..
        } => Some(format!("Running tool `{tool_name}` with args `{args}`...")),
        AgentEvent::ToolExecutionEnd {
            tool_name,
            result,
            is_error,
            ..
        } => {
            let details = tool_output_summary(result);
            if *is_error {
                Some(format!("Tool `{tool_name}` reported an error: {details}"))
            } else {
                Some(format!("Tool `{tool_name}` completed: {details}"))
            }
        }
        _ => None,
    }
}

fn tool_output_summary(output: &pi::tools::ToolOutput) -> String {
    let parts: Vec<&str> = output
        .content
        .iter()
        .filter_map(|block| {
            if let ContentBlock::Text(text) = block {
                let trimmed = text.text.trim();
                if !trimmed.is_empty() {
                    Some(trimmed)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    if parts.is_empty() {
        if output.is_error {
            "tool failed without details".to_string()
        } else {
            "ok".to_string()
        }
    } else {
        parts.join(" | ")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use pi::error::Result;
    use pi::model::{ContentBlock, TextContent};
    use pi::tools::{Tool, ToolOutput, ToolUpdate};
    use std::sync::{Arc, Mutex};

    struct EchoTool {
        name: String,
        last_input: Arc<Mutex<Option<serde_json::Value>>>,
    }

    impl EchoTool {
        fn new(name: &str) -> (Self, Arc<Mutex<Option<serde_json::Value>>>) {
            let last = Arc::new(Mutex::new(None));
            (
                Self {
                    name: name.to_string(),
                    last_input: Arc::clone(&last),
                },
                last,
            )
        }
    }

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn label(&self) -> &str {
            &self.name
        }
        fn description(&self) -> &str {
            "echo"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "text": { "type": "string" }
                },
                "required": ["task_id", "text"]
            })
        }
        async fn execute(
            &self,
            _id: &str,
            input: serde_json::Value,
            _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
        ) -> Result<ToolOutput> {
            *self.last_input.lock().unwrap() = Some(input.clone());
            Ok(ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new("ok".to_string()))],
                details: None,
                is_error: false,
            })
        }
    }

    #[test]
    fn needs_scoping_matches_expected_tools() {
        assert!(TaskScopedTool::needs_scoping("task_comment"));
        assert!(TaskScopedTool::needs_scoping("update_task_status"));
        assert!(TaskScopedTool::needs_scoping("update_task_labels"));
        assert!(!TaskScopedTool::needs_scoping("task_create"));
        assert!(!TaskScopedTool::needs_scoping("task_list"));
    }

    #[test]
    fn parameters_hides_task_id() {
        let (echo, _) = EchoTool::new("task_comment");
        let scoped = TaskScopedTool::new(Box::new(echo), "task-123".to_string());
        let params = scoped.parameters();
        let props = params["properties"].as_object().unwrap();
        assert!(!props.contains_key("task_id"), "task_id should be hidden");
        assert!(props.contains_key("text"), "text should remain");
        let required = params["required"].as_array().unwrap();
        assert!(!required.iter().any(|v| v == "task_id"));
    }

    #[tokio::test]
    async fn execute_injects_task_id() {
        let (echo, last_input) = EchoTool::new("task_comment");
        let scoped = TaskScopedTool::new(Box::new(echo), "task-abc".to_string());

        scoped
            .execute("call-1", serde_json::json!({"text": "hello"}), None)
            .await
            .unwrap();

        let captured = last_input.lock().unwrap().clone().unwrap();
        assert_eq!(captured["task_id"], "task-abc");
        assert_eq!(captured["text"], "hello");
    }
}
