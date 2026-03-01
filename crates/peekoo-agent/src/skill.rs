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
