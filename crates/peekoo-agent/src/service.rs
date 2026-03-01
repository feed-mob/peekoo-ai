//! Agent service — high-level wrapper around pi's `AgentSessionHandle`.
//!
//! Provides a simplified API for creating sessions, sending prompts,
//! and switching models at runtime.

use pi::sdk::{
    AgentEvent, AgentSessionHandle, AssistantMessage, ContentBlock, SessionOptions, SubscriptionId,
    create_agent_session,
};
use pi::error::Result;

use crate::config::AgentServiceConfig;

/// High-level agent service that wraps pi's session handle.
///
/// # Example
///
/// ```rust,no_run
/// use peekoo_agent::config::AgentServiceConfig;
/// use peekoo_agent::service::AgentService;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = AgentServiceConfig {
///     provider: Some("anthropic".into()),
///     model: Some("claude-sonnet-4-6".into()),
///     ..Default::default()
/// };
/// let mut agent = AgentService::new(config).await?;
/// let reply = agent.prompt("Hello!", |_event| {}).await?;
/// println!("{reply}");
/// # Ok(())
/// # }
/// ```
pub struct AgentService {
    handle: AgentSessionHandle,
}

impl AgentService {
    /// Create a new agent service with the given configuration.
    ///
    /// This initializes the LLM provider, loads tools (built-in + skills),
    /// and prepares the session for prompting.
    pub async fn new(config: AgentServiceConfig) -> Result<Self> {
        let default_config = pi::config::Config::load()?;

        // 1. Process Markdown Agent Skills (Prompts)
        let mut final_system_prompt = config.system_prompt.clone().unwrap_or_default();
        
        if !config.agent_skills.is_empty() {
            let options = pi::resources::LoadSkillsOptions {
                cwd: config.working_directory.clone(),
                agent_dir: pi::config::Config::global_dir(),
                skill_paths: config.agent_skills,
                include_defaults: false,
            };
            
            let loaded = pi::resources::load_skills(options);
            
            // Expose diagnostics/warnings to stderr if any failed to parse
            for diag in loaded.diagnostics {
                eprintln!("Warning (Markdown Skill): {}", diag.message);
            }
            
            if !loaded.skills.is_empty() {
                let skills_prompt = pi::resources::format_skills_for_prompt(&loaded.skills);
                if !final_system_prompt.is_empty() {
                    final_system_prompt.push_str("\n\n");
                }
                final_system_prompt.push_str(&skills_prompt);
            }
        }

        // 2. Determine LLM routing
        let provider_id = config
            .provider
            .clone()
            .unwrap_or_else(|| default_config.default_provider.clone().unwrap_or_else(|| "anthropic".to_string()));
        let model_id = config
            .model
            .clone()
            .unwrap_or_else(|| default_config.default_model.clone().unwrap_or_else(|| "claude-3-7-sonnet-latest".to_string()));

        let options = SessionOptions {
            provider: Some(provider_id),
            model: Some(model_id),
            api_key: config.api_key.clone(),
            system_prompt: if final_system_prompt.is_empty() { None } else { Some(final_system_prompt) },
            working_directory: Some(config.working_directory.clone()),
            max_tool_iterations: config.max_tool_iterations,
            no_session: true,
            enabled_tools: Some(pi::sdk::BUILTIN_TOOL_NAMES.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        };

        let handle = create_agent_session(options).await?;

        Ok(Self { handle })
    }

    /// Send a user prompt through the agent loop.
    ///
    /// The `on_event` callback fires for every [`AgentEvent`] (text deltas,
    /// tool calls, etc.) during streaming.
    ///
    /// Returns the final assistant message text.
    pub async fn prompt(
        &mut self,
        input: &str,
        on_event: impl Fn(AgentEvent) + Send + Sync + 'static,
    ) -> Result<String> {
        let assistant_msg = self.handle.prompt(input, on_event).await?;
        Ok(extract_text(&assistant_msg))
    }

    /// Send a user prompt and return the raw [`AssistantMessage`].
    pub async fn prompt_raw(
        &mut self,
        input: &str,
        on_event: impl Fn(AgentEvent) + Send + Sync + 'static,
    ) -> Result<AssistantMessage> {
        self.handle.prompt(input, on_event).await
    }

    /// Switch the active LLM provider and model at runtime.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example(agent: &mut peekoo_agent::service::AgentService) {
    /// agent.set_model("openai", "gpt-4o").await.unwrap();
    /// # }
    /// ```
    pub async fn set_model(&mut self, provider: &str, model: &str) -> Result<()> {
        self.handle.set_model(provider, model).await
    }

    /// Return the currently active `(provider, model)` pair.
    pub fn model(&self) -> (String, String) {
        self.handle.model()
    }

    /// Register a session-level event listener that fires for every prompt.
    pub fn subscribe(
        &self,
        listener: impl Fn(AgentEvent) + Send + Sync + 'static,
    ) -> SubscriptionId {
        self.handle.subscribe(listener)
    }

    /// Remove a previously registered event listener.
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        self.handle.unsubscribe(id)
    }

    /// Access the underlying pi session handle for advanced operations.
    pub fn handle(&self) -> &AgentSessionHandle {
        &self.handle
    }

    /// Mutable access to the underlying pi session handle.
    pub fn handle_mut(&mut self) -> &mut AgentSessionHandle {
        &mut self.handle
    }
}

/// Extract the concatenated text from an assistant message's content blocks.
fn extract_text(msg: &AssistantMessage) -> String {
    msg.content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pi::model::{TextContent, ThinkingContent, Usage, StopReason};

    fn make_message(content: Vec<ContentBlock>) -> AssistantMessage {
        AssistantMessage {
            content,
            api: "test".into(),
            provider: "test".into(),
            model: "test-model".into(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        }
    }

    #[test]
    fn extract_text_empty_message() {
        let msg = make_message(vec![]);
        assert_eq!(extract_text(&msg), "");
    }

    #[test]
    fn extract_text_single_block() {
        let msg = make_message(vec![ContentBlock::Text(TextContent::new("Hello!"))]);
        assert_eq!(extract_text(&msg), "Hello!");
    }

    #[test]
    fn extract_text_multiple_blocks_concatenates() {
        let msg = make_message(vec![
            ContentBlock::Text(TextContent::new("Hello ")),
            ContentBlock::Text(TextContent::new("world!")),
        ]);
        assert_eq!(extract_text(&msg), "Hello world!");
    }

    #[test]
    fn extract_text_skips_non_text_blocks() {
        let msg = make_message(vec![
            ContentBlock::Thinking(ThinkingContent {
                thinking: "hmm...".into(),
                thinking_signature: None,
            }),
            ContentBlock::Text(TextContent::new("The answer is 42.")),
        ]);
        assert_eq!(extract_text(&msg), "The answer is 42.");
    }

    #[test]
    fn extract_text_only_non_text_blocks_returns_empty() {
        let msg = make_message(vec![ContentBlock::Thinking(ThinkingContent {
            thinking: "thinking only".into(),
            thinking_signature: None,
        })]);
        assert_eq!(extract_text(&msg), "");
    }
}
