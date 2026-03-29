//! Agent service — high-level wrapper using the AgentBackend trait
//!
//! Provides a simplified API for creating sessions, sending prompts,
//! and switching providers/models at runtime via ACP-compatible agents.

use crate::backend::{AgentBackend, AgentEvent, BackendConfig, Message, MessageRole};
use crate::config::AgentServiceConfig;
use crate::mcp_bridge::McpBridge;
use crate::session_store::SessionStore;
use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;

/// High-level agent service using the backend trait
pub struct AgentService {
    /// The active backend (swappable at runtime)
    backend: Box<dyn AgentBackend>,
    /// Session store for persistence
    session_store: Option<SessionStore>,
    /// Current session ID
    session_id: Option<String>,
    /// MCP bridge for tool execution
    mcp_bridge: Option<McpBridge>,
    /// System prompt components
    system_prompt: String,
    /// Working directory
    working_directory: PathBuf,
    /// Configuration
    config: AgentServiceConfig,
}

impl AgentService {
    /// Create a new agent service with the given configuration
    pub async fn new(mut config: AgentServiceConfig) -> Result<Self> {
        // Resolve paths and auto-discovery
        Self::resolve_configuration(&mut config).await?;

        // Build system prompt from persona files + skills
        let system_prompt = Self::build_system_prompt(&config)?;

        // Initialize session store if persistence is enabled
        let session_store = if !config.no_session {
            let db_path = peekoo_paths::peekoo_settings_db_path().map_err(anyhow::Error::msg)?;

            // Ensure parent directory exists
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            Some(SessionStore::open(&db_path)?)
        } else {
            None
        };

        // Initialize MCP bridge if configured
        let mcp_bridge = if let Ok(mcp_url) = std::env::var("PEEKOO_MCP_URL") {
            let mut bridge = McpBridge::new(mcp_url);
            bridge.connect().await.ok(); // Best effort
            Some(bridge)
        } else {
            None
        };

        // Create backend based on provider
        let (command, args) = config
            .provider
            .command_with_environment(&config.environment);
        let mut backend = crate::backend::AcpBackend::new(command, args);

        // Initialize backend
        let backend_config = BackendConfig {
            working_directory: config.working_directory.clone(),
            system_prompt: Some(system_prompt.clone()),
            model: config.model.clone(),
            provider: Some(config.provider.id()),
            api_key: config.api_key.clone(),
            environment: config.environment.clone(),
            mcp_servers: config.mcp_servers.clone(),
        };

        backend.initialize(backend_config).await?;

        // Resume or create session
        let session_id = if let Some(resume_id) = &config.resume_session_id {
            // Try to resume existing session
            Self::resume_session(&mut backend, &session_store, resume_id).await?
        } else {
            // Create new session
            Self::create_new_session(&config, &session_store, &backend).await?
        };

        Ok(Self {
            backend: Box::new(backend),
            session_store,
            session_id: Some(session_id),
            mcp_bridge,
            system_prompt,
            working_directory: config.working_directory.clone(),
            config,
        })
    }

    /// Send a prompt and get response
    pub async fn prompt(
        &mut self,
        input: &str,
        on_event: impl Fn(AgentEvent) + Send + Sync + 'static,
    ) -> Result<String> {
        let session_id = self
            .session_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active session"))?;

        // Load conversation history
        let history = if let Some(store) = &self.session_store {
            store.load_messages(session_id)?
        } else {
            vec![]
        };

        // Create user message
        let user_message = Message {
            role: MessageRole::User,
            content: vec![crate::backend::ContentBlock::Text {
                text: input.to_string(),
            }],
            tool_calls: None,
            tool_call_id: None,
        };

        // Send prompt to backend
        let result = self
            .backend
            .prompt(input, history, Box::new(on_event))
            .await?;

        // Save messages to session store
        if let Some(store) = &self.session_store {
            // Save user message
            store.append_message(
                session_id,
                &user_message,
                Some(&self.backend.current_model().provider),
                None,
                None,
            )?;

            // Save assistant response
            let assistant_message = Message {
                role: MessageRole::Assistant,
                content: vec![crate::backend::ContentBlock::Text {
                    text: result.content.clone(),
                }],
                tool_calls: None,
                tool_call_id: None,
            };

            store.append_message(
                session_id,
                &assistant_message,
                Some(&self.backend.current_model().provider),
                Some(&self.backend.current_model().model),
                result.usage.as_ref(),
            )?;

            // Update provider state if available
            if let Some(state) = &result.provider_state {
                store.update_provider_state(session_id, state)?;
            }
        }

        Ok(result.content)
    }

    /// Switch to a different provider/model at runtime
    pub async fn set_provider(
        &mut self,
        provider: crate::config::AgentProvider,
        model: Option<String>,
    ) -> Result<()> {
        let session_id = self
            .session_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active session"))?;

        let (command, args) = provider.command_with_environment(&self.config.environment);
        let provider_id = provider.id();

        // Clone for storage before moving to backend
        let command_for_storage = command.clone();
        let args_for_storage = args.clone();

        // Create new backend
        let mut new_backend = crate::backend::AcpBackend::new(command, args);

        // Initialize with current configuration
        let backend_config = BackendConfig {
            working_directory: self.working_directory.clone(),
            system_prompt: Some(self.system_prompt.clone()),
            model: model.clone(),
            provider: Some(provider_id.clone()),
            api_key: self.config.api_key.clone(),
            environment: self.config.environment.clone(),
            mcp_servers: self.config.mcp_servers.clone(),
        };

        new_backend.initialize(backend_config).await?;

        // Switch backends
        self.backend = Box::new(new_backend);
        self.config.provider = provider;
        self.config.model = model;

        // Update session store with new provider
        if let Some(store) = &self.session_store {
            store.switch_provider(
                session_id,
                &provider_id,
                &command_for_storage,
                &args_for_storage,
            )?;
        }

        Ok(())
    }

    /// Get current model information
    pub fn current_model(&self) -> crate::backend::ModelInfo {
        self.backend.current_model()
    }

    /// Get current model as (provider, model) tuple
    pub fn model(&self) -> (String, String) {
        let info = self.backend.current_model();
        (info.provider, info.model)
    }

    /// Switch to a different model/provider at runtime
    pub async fn set_model(&mut self, provider: &str, model: &str) -> Result<()> {
        self.backend.set_model(provider, model).await
    }

    /// Cancel in-flight prompt
    pub async fn cancel(&self) -> Result<()> {
        self.backend.cancel().await
    }

    /// Close the session
    pub async fn close(self) -> Result<()> {
        if let (Some(session_id), Some(store)) = (&self.session_id, &self.session_store) {
            store.update_session_status(session_id, "closed")?;
        }
        Ok(())
    }

    /// Get session information
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Get session file path
    pub fn session_path(&self) -> Option<std::path::PathBuf> {
        self.session_store.as_ref().map(|store| store.db_path())
    }

    /// Check if session persistence is enabled
    pub fn has_persistence(&self) -> bool {
        self.session_store.is_some()
    }

    /// Get conversation messages as JSON strings
    pub fn messages_json(&self) -> Vec<String> {
        if let (Some(store), Some(session_id)) = (&self.session_store, &self.session_id) {
            match store.load_messages(session_id) {
                Ok(messages) => messages
                    .into_iter()
                    .map(|msg| serde_json::to_string(&msg).unwrap_or_default())
                    .collect(),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }

    // Helper methods

    async fn resolve_configuration(config: &mut AgentServiceConfig) -> Result<()> {
        // Resolve paths from auto-discovery if enabled
        if config.auto_discover {
            let mut search_paths = Vec::new();

            // Environment variable
            if let Ok(env_dir) = std::env::var("PEEKOO_CONFIG_DIR") {
                search_paths.push(PathBuf::from(env_dir));
            }

            // Local .peekoo directory
            search_paths.push(config.working_directory.join(".peekoo"));

            // Global config
            if let Ok(global_dir) = peekoo_paths::peekoo_global_config_dir() {
                search_paths.push(global_dir);
            }

            for path in search_paths {
                if path.is_dir() {
                    if config.persona_dir.is_none() {
                        config.persona_dir = Some(path.clone());
                    }

                    if config.agent_skills.is_empty() {
                        let skills_dir = path.join("skills");
                        if skills_dir.is_dir() {
                            config.agent_skills.push(skills_dir);
                        }
                    }

                    config.working_directory = path.clone();
                    break;
                }
            }
        }

        Ok(())
    }

    fn build_system_prompt(config: &AgentServiceConfig) -> Result<String> {
        let mut sections = Vec::new();

        if let Some(persona_dir) = config.persona_dir.as_ref() {
            sections.extend(load_persona_sections(persona_dir)?);
        }

        if let Some(prompt) = config.system_prompt.as_ref().map(|prompt| prompt.trim())
            && !prompt.is_empty()
        {
            sections.push(("System Prompt".to_string(), prompt.to_string()));
        }

        let skill_sections = load_skill_sections(&config.agent_skills)?;
        if !skill_sections.is_empty() {
            sections.extend(
                skill_sections
                    .into_iter()
                    .map(|(name, content)| (format!("Skill: {name}"), content)),
            );
        }

        if sections.is_empty() {
            return Ok("You are a helpful assistant.".to_string());
        }

        Ok(sections
            .into_iter()
            .map(|(title, content)| format!("## {title}\n{content}"))
            .collect::<Vec<_>>()
            .join("\n\n"))
    }

    async fn create_new_session(
        config: &AgentServiceConfig,
        session_store: &Option<SessionStore>,
        _backend: &dyn AgentBackend,
    ) -> Result<String> {
        let (command, args) = config
            .provider
            .command_with_environment(&config.environment);

        let skills: Vec<String> = config
            .agent_skills
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let session_id = if let Some(store) = session_store {
            let session_id = store.create_session(
                None, // title
                &config.provider.id(),
                &command,
                &args,
                &config.working_directory,
                config.system_prompt.as_deref(),
                &skills,
            )?;
            store.update_runtime_context(
                &session_id,
                config.llm_provider_id.as_deref(),
                config.model.as_deref(),
            )?;
            session_id
        } else {
            generate_temp_session_id()
        };

        Ok(session_id)
    }

    async fn resume_session(
        backend: &mut dyn AgentBackend,
        session_store: &Option<SessionStore>,
        session_id: &str,
    ) -> Result<String> {
        if let Some(store) = session_store {
            let session = store
                .load_session(session_id)?
                .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;

            // Restore provider state if available
            if let Some(state_json) = session.provider_state {
                backend.restore_provider_state(state_json).await.ok(); // Best effort
            }

            // Update session status
            store.update_session_status(session_id, "active")?;
        }

        Ok(session_id.to_string())
    }
}

fn load_persona_sections(persona_dir: &std::path::Path) -> Result<Vec<(String, String)>> {
    let mut sections = Vec::new();

    let ordered_files = [
        ("AGENTS", "AGENTS.md"),
        ("BOOTSTRAP", "BOOTSTRAP.md"),
        ("SOUL", "SOUL.md"),
        ("IDENTITY", "IDENTITY.md"),
        ("USER", "USER.md"),
    ];

    for (title, file_name) in ordered_files {
        if let Some(content) = read_markdown_file(&persona_dir.join(file_name))? {
            sections.push((title.to_string(), content));
        }
    }

    for memory_name in ["MEMORY.md", "memory.md"] {
        if let Some(content) = read_markdown_file(&persona_dir.join(memory_name))? {
            sections.push(("MEMORY".to_string(), content));
            break;
        }
    }

    let memories_dir = persona_dir.join("memories");
    if memories_dir.is_dir() {
        let mut memory_files = std::fs::read_dir(&memories_dir)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "md"))
            .collect::<Vec<_>>();
        memory_files.sort();

        for path in memory_files {
            if let Some(content) = read_markdown_file(&path)? {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("memory");
                sections.push((format!("MEMORY: {name}"), content));
            }
        }
    }

    Ok(sections)
}

fn load_skill_sections(skill_paths: &[PathBuf]) -> Result<Vec<(String, String)>> {
    let mut sections = Vec::new();
    let mut seen = HashSet::new();

    for skill_path in skill_paths {
        load_skill_sections_from_path(skill_path, &mut seen, &mut sections)?;
    }

    Ok(sections)
}

fn load_skill_sections_from_path(
    skill_path: &std::path::Path,
    seen: &mut HashSet<PathBuf>,
    sections: &mut Vec<(String, String)>,
) -> Result<()> {
    let canonical = std::fs::canonicalize(skill_path).unwrap_or_else(|_| skill_path.to_path_buf());
    if !seen.insert(canonical) {
        return Ok(());
    }

    if skill_path.is_file() {
        if let Some(content) = read_markdown_file(skill_path)? {
            let name = skill_path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("skill");
            sections.push((name.to_string(), content));
        }
        return Ok(());
    }

    if !skill_path.is_dir() {
        return Ok(());
    }

    let direct_skill = skill_path.join("SKILL.md");
    if direct_skill.is_file() {
        if let Some(content) = read_markdown_file(&direct_skill)? {
            let name = skill_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("skill");
            sections.push((name.to_string(), content));
        }
        return Ok(());
    }

    let mut entries = std::fs::read_dir(skill_path)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect::<Vec<_>>();
    entries.sort();

    for entry in entries {
        load_skill_sections_from_path(&entry, seen, sections)?;
    }

    Ok(())
}

fn read_markdown_file(path: &std::path::Path) -> Result<Option<String>> {
    if !path.is_file() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(path)?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Ok(Some(trimmed.to_string()))
}

fn generate_temp_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("temp_sess_{}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_service_creation() {
        // This test requires a running ACP agent, so we'll just test config
        let config = AgentServiceConfig::default();

        assert!(config.provider.id().len() > 0);
    }

    #[test]
    fn test_system_prompt_building() {
        let config = AgentServiceConfig {
            system_prompt: Some("Custom prompt".to_string()),
            ..Default::default()
        };

        let prompt = AgentService::build_system_prompt(&config).unwrap();
        assert_eq!(prompt, "## System Prompt\nCustom prompt");
    }

    #[test]
    fn test_default_system_prompt() {
        let config = AgentServiceConfig::default();

        let prompt = AgentService::build_system_prompt(&config).unwrap();
        assert_eq!(prompt, "You are a helpful assistant.");
    }

    #[test]
    fn test_generate_temp_session_id() {
        let id1 = generate_temp_session_id();
        std::thread::sleep(std::time::Duration::from_millis(2)); // Ensure different timestamps
        let id2 = generate_temp_session_id();

        assert!(id1.starts_with("temp_sess_"));
        assert!(id2.starts_with("temp_sess_"));
        assert_ne!(id1, id2); // Should be unique
    }

    #[test]
    fn test_system_prompt_includes_persona_files_in_documented_order() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("SOUL.md"), "Soul content").unwrap();
        std::fs::write(temp_dir.path().join("AGENTS.md"), "Agents content").unwrap();
        std::fs::write(temp_dir.path().join("MEMORY.md"), "Memory content").unwrap();
        std::fs::create_dir_all(temp_dir.path().join("memories")).unwrap();
        std::fs::write(
            temp_dir.path().join("memories").join("project.md"),
            "Project memory",
        )
        .unwrap();

        let config = AgentServiceConfig {
            persona_dir: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };

        let prompt = AgentService::build_system_prompt(&config).unwrap();
        let expected = [
            "## AGENTS\nAgents content",
            "## SOUL\nSoul content",
            "## MEMORY\nMemory content",
            "## MEMORY: project.md\nProject memory",
        ];

        for section in expected {
            assert!(prompt.contains(section), "missing section: {section}");
        }

        assert!(prompt.find("## AGENTS").unwrap() < prompt.find("## SOUL").unwrap());
        assert!(prompt.find("## SOUL").unwrap() < prompt.find("## MEMORY").unwrap());
    }

    #[test]
    fn test_system_prompt_loads_skill_files_and_skill_directories() {
        let temp_dir = tempfile::tempdir().unwrap();
        let direct_skill = temp_dir.path().join("direct-skill.md");
        let skill_dir = temp_dir.path().join("skill-dir");
        std::fs::write(&direct_skill, "Direct skill").unwrap();
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "Directory skill").unwrap();

        let config = AgentServiceConfig {
            agent_skills: vec![direct_skill, skill_dir],
            ..Default::default()
        };

        let prompt = AgentService::build_system_prompt(&config).unwrap();
        assert!(prompt.contains("## Skill: direct-skill\nDirect skill"));
        assert!(prompt.contains("## Skill: skill-dir\nDirectory skill"));
    }
}
