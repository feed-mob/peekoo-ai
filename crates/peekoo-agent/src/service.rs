//! Agent service — high-level wrapper around pi's `AgentSessionHandle`.
//!
//! Provides a simplified API for creating sessions, sending prompts,
//! and switching models at runtime.

use pi::error::Result;
use pi::sdk::{
    AgentEvent, AgentSessionHandle, AssistantMessage, ContentBlock, SessionOptions, SubscriptionId,
    create_agent_session,
};

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
    pub async fn new(mut config: AgentServiceConfig) -> Result<Self> {
        let default_config = pi::config::Config::load()?;

        // Resolve paths from auto-discovery if enabled
        let mut resolved_persona_dir = config.persona_dir.clone();
        let mut resolved_agent_skills = config.agent_skills.clone();

        if config.auto_discover {
            let mut search_paths = Vec::new();

            // 1. Highest priority: Environment variable
            if let Ok(env_dir) = std::env::var("PEEKOO_CONFIG_DIR") {
                search_paths.push(std::path::PathBuf::from(env_dir));
            }

            // 2. Next: Local working directory (crawled up to workspace root)
            search_paths.push(config.working_directory.join(".peekoo"));

            // 3. Last fallback: Global ~/.peekoo
            if let Some(home) = dirs::home_dir() {
                search_paths.push(home.join(".peekoo"));
            }

            for path in search_paths {
                if path.is_dir() {
                    // Only auto-discover persona if not explicitly set
                    if resolved_persona_dir.is_none() {
                        resolved_persona_dir = Some(path.clone());
                    }

                    // Only auto-discover skills if none explicitly set
                    if resolved_agent_skills.is_empty() {
                        let skills_dir = path.join("skills");
                        if skills_dir.is_dir() {
                            resolved_agent_skills.push(skills_dir);
                        }
                    }

                    // Peekoo is an assistant sprite, so its execution workspace
                    // is isolated inside its configuration directory.
                    let workspace_dir = path.join("workspace");
                    if !workspace_dir.exists() {
                        let _ = std::fs::create_dir_all(&workspace_dir);
                    }
                    config.working_directory = workspace_dir;

                    // If we found a config dir (either local or global), stop searching.
                    // The first match completely overrides any further ones
                    // to prevent mixing local personas with global skills.
                    break;
                }
            }
        }

        // 1. Compose system prompt from persona files + user prompt + skills
        let mut prompt_parts: Vec<String> = Vec::new();

        // 1a. Load OpenClaw-style persona files (IDENTITY → SOUL → MEMORY)
        if let Some(ref persona_dir) = resolved_persona_dir {
            // Identity and Soul
            for (filename, label) in &[("IDENTITY.md", "Identity"), ("SOUL.md", "Soul")] {
                let path = persona_dir.join(filename);
                if path.is_file()
                    && let Ok(content) = std::fs::read_to_string(&path)
                    && !content.trim().is_empty()
                {
                    prompt_parts.push(format!("## {label}\n\n{}", content.trim()));
                }
            }

            // Memory files (memory.md, MEMORY.md, and memories/*.md)
            let mut memory_parts = Vec::new();

            // Core memory file
            for core_mem in &["memory.md", "MEMORY.md"] {
                let path = persona_dir.join(core_mem);
                if path.is_file()
                    && let Ok(content) = std::fs::read_to_string(&path)
                    && !content.trim().is_empty()
                {
                    memory_parts.push(content.trim().to_string());
                    break; // Only load one core memory file
                }
            }

            // Topic memory files
            let memories_dir = persona_dir.join("memories");
            if memories_dir.is_dir()
                && let Ok(entries) = std::fs::read_dir(&memories_dir)
            {
                let mut mem_files: Vec<_> = entries
                    .filter_map(|r| r.ok())
                    .map(|e| e.path())
                    .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "md"))
                    .collect();
                mem_files.sort(); // Consistent ordering

                for path in mem_files {
                    if let Ok(content) = std::fs::read_to_string(&path)
                        && !content.trim().is_empty()
                    {
                        let title = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .replace(['_', '-'], " ");
                        memory_parts.push(format!("### {}\n{}", title, content.trim()));
                    }
                }
            }

            if !memory_parts.is_empty() {
                prompt_parts.push(format!("## Memory\n\n{}", memory_parts.join("\n\n")));
            }
        }

        // 1b. Append user-provided system prompt
        if let Some(ref user_prompt) = config.system_prompt
            && !user_prompt.trim().is_empty()
        {
            prompt_parts.push(user_prompt.trim().to_string());
        }

        // 1c. Append agent skills (markdown skill instructions)
        let mut final_system_prompt = prompt_parts.join("\n\n");

        if !resolved_agent_skills.is_empty() {
            let options = pi::resources::LoadSkillsOptions {
                cwd: config.working_directory.clone(),
                agent_dir: pi::config::Config::global_dir(),
                skill_paths: resolved_agent_skills,
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
        let provider_id = config.provider.clone().unwrap_or_else(|| {
            default_config
                .default_provider
                .clone()
                .unwrap_or_else(|| "anthropic".to_string())
        });
        let model_id = config.model.clone().unwrap_or_else(|| {
            default_config
                .default_model
                .clone()
                .unwrap_or_else(|| "claude-3-7-sonnet-latest".to_string())
        });

        let options = SessionOptions {
            provider: Some(provider_id),
            model: Some(model_id),
            api_key: config.api_key.clone(),
            system_prompt: if final_system_prompt.is_empty() {
                None
            } else {
                Some(final_system_prompt)
            },
            working_directory: Some(config.working_directory.clone()),
            max_tool_iterations: config.max_tool_iterations,
            no_session: true,
            enabled_tools: Some(
                pi::sdk::BUILTIN_TOOL_NAMES
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            ),
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
    use pi::model::{StopReason, TextContent, ThinkingContent, Usage};

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
