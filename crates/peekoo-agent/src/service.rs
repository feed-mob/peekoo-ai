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
use crate::skill::SkillAdapter;

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
        // Build the list of enabled tools — all built-in tools are enabled.
        let enabled_tools: Vec<String> = pi::sdk::BUILTIN_TOOL_NAMES
            .iter()
            .map(|s| s.to_string())
            .collect();

        // Collect skill names so pi knows about them.
        let skill_names: Vec<String> = config.skills.iter().map(|s| s.name().to_string()).collect();
        let mut all_tool_names = enabled_tools.clone();
        all_tool_names.extend(skill_names.clone());

        let options = SessionOptions {
            provider: config.provider,
            model: config.model,
            api_key: config.api_key,
            system_prompt: config.system_prompt,
            working_directory: Some(config.working_directory),
            no_session: true,
            max_tool_iterations: config.max_tool_iterations,
            enabled_tools: Some(enabled_tools),
            ..SessionOptions::default()
        };

        let mut handle = create_agent_session(options).await?;

        // Register custom skills as tools on the agent.
        let skill_tools: Vec<Box<dyn pi::tools::Tool>> = config
            .skills
            .into_iter()
            .map(|skill| Box::new(SkillAdapter::new(skill)) as Box<dyn pi::tools::Tool>)
            .collect();
        handle.session_mut().agent.extend_tools(skill_tools);

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
