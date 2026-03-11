//! Agent service — high-level wrapper around pi's `AgentSessionHandle`.
//!
//! Provides a simplified API for creating sessions, sending prompts,
//! and switching models at runtime.

use pi::error::Result;
use pi::sdk::{
    AgentEvent, AgentSessionHandle, AssistantMessage, ContentBlock, SessionOptions, SubscriptionId,
    create_agent_session,
};
use std::path::Path;

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

            // 3. Last fallback: global peekoo config dir, then legacy ~/.peekoo
            if let Ok(global_dir) = peekoo_paths::peekoo_global_config_dir() {
                search_paths.push(global_dir);
            }
            if let Some(legacy_home) = peekoo_paths::peekoo_legacy_home_dir_if_distinct() {
                search_paths.push(legacy_home);
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
        let prompt_parts = compose_prompt_parts(
            resolved_persona_dir.as_deref(),
            config.system_prompt.as_deref(),
        );

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
            no_session: config.no_session,
            session_dir: config.session_dir.clone(),
            session_path: config.session_path.clone(),
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

    /// Return the current conversation messages as serialised JSON values.
    ///
    /// Each element is a `serde_json::Value` representing a [`pi::model::Message`]
    /// (tagged by `role`). Returns an empty `Vec` when there is no history.
    pub fn messages_json(&self) -> Vec<serde_json::Value> {
        let session = self.handle.session();
        let messages = session.agent.messages();
        messages
            .iter()
            .filter_map(|m| serde_json::to_value(m).ok())
            .collect()
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

fn read_non_empty_markdown(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }

    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn load_named_section(persona_dir: &Path, filename: &str, label: &str) -> Option<String> {
    let content = read_non_empty_markdown(&persona_dir.join(filename))?;
    Some(format!("## {label}\n\n{content}"))
}

fn load_memory_section(persona_dir: &Path) -> Option<String> {
    let mut memory_parts = Vec::new();

    for core_mem in &["memory.md", "MEMORY.md"] {
        if let Some(content) = read_non_empty_markdown(&persona_dir.join(core_mem)) {
            memory_parts.push(content);
            break;
        }
    }

    let memories_dir = persona_dir.join("memories");
    if memories_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&memories_dir)
    {
        let mut mem_files: Vec<_> = entries
            .filter_map(|r| r.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "md"))
            .collect();
        mem_files.sort();

        for path in mem_files {
            if let Some(content) = read_non_empty_markdown(&path) {
                let title = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .replace(['_', '-'], " ");
                memory_parts.push(format!("### {title}\n{content}"));
            }
        }
    }

    if memory_parts.is_empty() {
        None
    } else {
        Some(format!("## Memory\n\n{}", memory_parts.join("\n\n")))
    }
}

fn compose_prompt_parts(persona_dir: Option<&Path>, user_prompt: Option<&str>) -> Vec<String> {
    let mut prompt_parts: Vec<String> = Vec::new();

    if let Some(persona_dir) = persona_dir {
        for (filename, label) in &[("AGENTS.md", "Agents"), ("SOUL.md", "Soul")] {
            if let Some(section) = load_named_section(persona_dir, filename, label) {
                prompt_parts.push(section);
            }
        }

        for (filename, label) in &[("IDENTITY.md", "Identity"), ("USER.md", "User")] {
            if let Some(section) = load_named_section(persona_dir, filename, label) {
                prompt_parts.push(section);
            }
        }

        if let Some(memory) = load_memory_section(persona_dir) {
            prompt_parts.push(memory);
        }
    }

    if let Some(user_prompt) = user_prompt
        && !user_prompt.trim().is_empty()
    {
        prompt_parts.push(user_prompt.trim().to_string());
    }

    prompt_parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use pi::model::{StopReason, TextContent, ThinkingContent, Usage};
    use std::path::{Path, PathBuf};

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

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        path.push(format!("peekoo-agent-{prefix}-{nanos}"));
        std::fs::create_dir_all(&path).expect("create temp test dir");
        path
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent directory");
        }
        std::fs::write(path, content).expect("write test file");
    }

    #[test]
    fn compose_prompt_parts_without_agents_still_orders_sections_consistently() {
        let dir = temp_test_dir("no-agents-order");
        write_file(&dir.join("IDENTITY.md"), "Identity content");
        write_file(&dir.join("SOUL.md"), "Soul content");
        write_file(&dir.join("memory.md"), "Core memory");

        let parts = compose_prompt_parts(Some(&dir), Some("User prompt"));
        assert_eq!(parts.len(), 4);
        assert!(parts[0].starts_with("## Soul\n\nSoul content"));
        assert!(parts[1].starts_with("## Identity\n\nIdentity content"));
        assert!(parts[2].starts_with("## Memory\n\nCore memory"));
        assert_eq!(parts[3], "User prompt");
    }

    #[test]
    fn compose_prompt_parts_uses_new_startup_order() {
        let dir = temp_test_dir("agents-user-order");
        write_file(&dir.join("IDENTITY.md"), "Identity content");
        write_file(&dir.join("SOUL.md"), "Soul content");
        write_file(&dir.join("memory.md"), "Core memory");
        write_file(&dir.join("AGENTS.md"), "Agent instructions");
        write_file(&dir.join("USER.md"), "User profile");

        let parts = compose_prompt_parts(Some(&dir), Some("User prompt"));
        assert_eq!(parts.len(), 6);
        assert!(parts[0].starts_with("## Agents\n\nAgent instructions"));
        assert!(parts[1].starts_with("## Soul\n\nSoul content"));
        assert!(parts[2].starts_with("## Identity\n\nIdentity content"));
        assert!(parts[3].starts_with("## User\n\nUser profile"));
        assert!(parts[4].starts_with("## Memory\n\nCore memory"));
        assert_eq!(parts[5], "User prompt");
    }

    #[test]
    fn compose_prompt_parts_skips_empty_agents_and_user_files() {
        let dir = temp_test_dir("empty-startup");
        write_file(&dir.join("AGENTS.md"), "   \n");
        write_file(&dir.join("USER.md"), "\n");

        let parts = compose_prompt_parts(Some(&dir), None);
        assert!(parts.is_empty());
    }

    #[test]
    fn compose_prompt_parts_supports_agents_only() {
        let dir = temp_test_dir("agents-only");
        write_file(&dir.join("AGENTS.md"), "Agent instructions");

        let parts = compose_prompt_parts(Some(&dir), None);
        assert_eq!(parts.len(), 1);
        assert!(parts[0].starts_with("## Agents\n\nAgent instructions"));
    }

    #[test]
    fn compose_prompt_parts_supports_user_only() {
        let dir = temp_test_dir("user-only");
        write_file(&dir.join("USER.md"), "User profile");

        let parts = compose_prompt_parts(Some(&dir), None);
        assert_eq!(parts.len(), 1);
        assert!(parts[0].starts_with("## User\n\nUser profile"));
    }

    #[test]
    fn compose_prompt_parts_keeps_memory_precedence() {
        let dir = temp_test_dir("memory-precedence");
        write_file(&dir.join("memory.md"), "lowercase wins");
        write_file(&dir.join("MEMORY.md"), "uppercase ignored");

        let parts = compose_prompt_parts(Some(&dir), None);
        assert_eq!(parts.len(), 1);
        assert!(parts[0].contains("lowercase wins"));
        assert!(!parts[0].contains("uppercase ignored"));
    }

    #[test]
    fn compose_prompt_parts_sorts_topic_memories() {
        let dir = temp_test_dir("memories-sorted");
        write_file(&dir.join("memories/zeta.md"), "zeta");
        write_file(&dir.join("memories/alpha.md"), "alpha");

        let parts = compose_prompt_parts(Some(&dir), None);
        assert_eq!(parts.len(), 1);
        let memory = &parts[0];
        let alpha_index = memory
            .find("### alpha\nalpha")
            .expect("alpha section present");
        let zeta_index = memory.find("### zeta\nzeta").expect("zeta section present");
        assert!(alpha_index < zeta_index);
    }
}
