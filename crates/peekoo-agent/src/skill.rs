//! Skill abstraction — domain-specific tools that extend the agent.
//!
//! A [`Skill`] is a peekoo-specific tool. At runtime, skills are wrapped via
//! [`SkillAdapter`] into pi's [`pi::tools::Tool`] trait so the agent loop can
//! call them transparently alongside built-in tools.

use async_trait::async_trait;
use pi::error::Result;
use pi::tools::{Tool, ToolOutput, ToolUpdate};
use pi::model::{ContentBlock, TextContent};
use serde_json::Value;

// ============================================================================
// Skill trait
// ============================================================================

/// A domain-specific tool that can be registered with the agent.
///
/// Implement this trait to add custom capabilities — calendar lookups,
/// project queries, Pomodoro controls, etc. — and pass them via
/// [`AgentServiceConfig::skills`](super::config::AgentServiceConfig).
#[async_trait]
pub trait Skill: Send + Sync {
    /// Machine-readable tool name (e.g. `"calendar_lookup"`).
    fn name(&self) -> &str;

    /// Human-readable display label.
    fn label(&self) -> &str {
        self.name()
    }

    /// Description shown to the LLM so it knows when to invoke this tool.
    fn description(&self) -> &str;

    /// JSON Schema describing the expected input arguments.
    fn parameters(&self) -> Value;

    /// Execute the skill with the given arguments and return a text result.
    async fn execute(&self, args: Value) -> Result<String>;

    /// Whether the skill is read-only (safe to run in parallel).
    fn is_read_only(&self) -> bool {
        true
    }
}

// ============================================================================
// SkillAdapter — bridges Skill → pi::Tool
// ============================================================================

/// Wraps a [`Skill`] so it satisfies pi's [`Tool`] trait.
pub struct SkillAdapter {
    skill: Box<dyn Skill>,
}

impl SkillAdapter {
    pub fn new(skill: Box<dyn Skill>) -> Self {
        Self { skill }
    }
}

#[async_trait]
impl Tool for SkillAdapter {
    fn name(&self) -> &str {
        self.skill.name()
    }

    fn label(&self) -> &str {
        self.skill.label()
    }

    fn description(&self) -> &str {
        self.skill.description()
    }

    fn parameters(&self) -> Value {
        self.skill.parameters()
    }

    async fn execute(
        &self,
        _tool_call_id: &str,
        input: Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        match self.skill.execute(input).await {
            Ok(text) => Ok(ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(text))],
                details: None,
                is_error: false,
            }),
            Err(e) => Ok(ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(format!("Error: {e}")))],
                details: None,
                is_error: true,
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        self.skill.is_read_only()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pi::model::ContentBlock;

    // ── Mock Skill ─────────────────────────────────────────────────

    struct EchoSkill;

    #[async_trait]
    impl Skill for EchoSkill {
        fn name(&self) -> &str {
            "echo"
        }

        fn label(&self) -> &str {
            "Echo Tool"
        }

        fn description(&self) -> &str {
            "Echoes back the input message"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"]
            })
        }

        async fn execute(&self, args: Value) -> Result<String> {
            let msg = args
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("(empty)");
            Ok(format!("echo: {msg}"))
        }

        fn is_read_only(&self) -> bool {
            true
        }
    }

    struct FailingSkill;

    #[async_trait]
    impl Skill for FailingSkill {
        fn name(&self) -> &str {
            "fail"
        }

        fn description(&self) -> &str {
            "Always fails"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({})
        }

        async fn execute(&self, _args: Value) -> Result<String> {
            Err(pi::error::Error::validation("intentional test error"))
        }

        fn is_read_only(&self) -> bool {
            false
        }
    }

    // ── Skill defaults ─────────────────────────────────────────────

    struct MinimalSkill;

    #[async_trait]
    impl Skill for MinimalSkill {
        fn name(&self) -> &str {
            "minimal"
        }

        fn description(&self) -> &str {
            "A minimal skill"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({})
        }

        async fn execute(&self, _args: Value) -> Result<String> {
            Ok("ok".into())
        }
    }

    #[test]
    fn skill_default_label_is_name() {
        let skill = MinimalSkill;
        assert_eq!(skill.label(), skill.name());
    }

    #[test]
    fn skill_default_is_read_only() {
        let skill = MinimalSkill;
        assert!(skill.is_read_only());
    }

    // ── Adapter metadata pass-through ──────────────────────────────

    #[test]
    fn adapter_delegates_name_and_label() {
        let adapter = SkillAdapter::new(Box::new(EchoSkill));
        assert_eq!(Tool::name(&adapter), "echo");
        assert_eq!(Tool::label(&adapter), "Echo Tool");
    }

    #[test]
    fn adapter_delegates_description() {
        let adapter = SkillAdapter::new(Box::new(EchoSkill));
        assert_eq!(Tool::description(&adapter), "Echoes back the input message");
    }

    #[test]
    fn adapter_delegates_parameters() {
        let adapter = SkillAdapter::new(Box::new(EchoSkill));
        let params = Tool::parameters(&adapter);
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["message"].is_object());
    }

    #[test]
    fn adapter_delegates_is_read_only() {
        let echo = SkillAdapter::new(Box::new(EchoSkill));
        assert!(Tool::is_read_only(&echo));

        let fail = SkillAdapter::new(Box::new(FailingSkill));
        assert!(!Tool::is_read_only(&fail));
    }

    // ── Adapter execution ──────────────────────────────────────────

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        // Use pi's async runtime for testing.
        let reactor =
            asupersync::runtime::reactor::create_reactor().expect("create reactor");
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .with_reactor(reactor)
            .build()
            .expect("build runtime");
        runtime.block_on(f)
    }

    #[test]
    fn adapter_execute_success() {
        let adapter = SkillAdapter::new(Box::new(EchoSkill));
        let input = serde_json::json!({ "message": "hello world" });

        let output = block_on(adapter.execute("call-1", input, None)).expect("execute");

        assert!(!output.is_error);
        assert_eq!(output.content.len(), 1);
        match &output.content[0] {
            ContentBlock::Text(t) => assert_eq!(t.text, "echo: hello world"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn adapter_execute_error_is_marked() {
        let adapter = SkillAdapter::new(Box::new(FailingSkill));

        let output =
            block_on(adapter.execute("call-2", serde_json::json!({}), None)).expect("execute");

        assert!(output.is_error);
        assert_eq!(output.content.len(), 1);
        match &output.content[0] {
            ContentBlock::Text(t) => assert!(
                t.text.contains("Error:"),
                "expected error text, got: {}",
                t.text
            ),
            other => panic!("expected Text, got {other:?}"),
        }
    }
}
