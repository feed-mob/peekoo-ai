use std::sync::Mutex;

use peekoo_agent::AgentEvent;
use peekoo_agent::config::AgentServiceConfig;
use peekoo_agent::service::AgentService;

use crate::settings::{
    AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto, OauthCancelResponse,
    OauthStartResponse, OauthStatusRequest, OauthStatusResponse, ProviderAuthDto,
    ProviderConfigDto, ProviderRequest, SetApiKeyRequest, SetProviderConfigRequest,
    SettingsService,
};

pub struct AgentApplication {
    agent: Mutex<Option<AgentService>>,
    settings: SettingsService,
    agent_config_version: Mutex<Option<i64>>,
}

impl AgentApplication {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            agent: Mutex::new(None),
            settings: SettingsService::new()?,
            agent_config_version: Mutex::new(None),
        })
    }

    pub async fn prompt_streaming<F>(&self, message: &str, on_event: F) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static,
    {
        let mut agent = {
            let mut guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
            let mut version_guard = self
                .agent_config_version
                .lock()
                .map_err(|e| format!("Version lock error: {e}"))?;

            let should_recreate = guard.is_none();

            if should_recreate {
                let config = self.resolved_config()?;
                let (config, settings_version) = self.settings.to_agent_config(config)?;

                let reactor = asupersync::runtime::reactor::create_reactor()
                    .map_err(|e| format!("Reactor error: {e}"))?;
                let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
                    .with_reactor(reactor)
                    .build()
                    .map_err(|e| format!("Runtime error: {e}"))?;

                let service = runtime
                    .block_on(AgentService::new(config))
                    .map_err(|e| format!("Agent init error: {e}"))?;
                *guard = Some(service);
                *version_guard = Some(settings_version);
            } else {
                let current_version = self.settings.get_settings()?.version;
                if (*version_guard) != Some(current_version) {
                    let config = self.resolved_config()?;
                    let (config, settings_version) = self.settings.to_agent_config(config)?;

                    let reactor = asupersync::runtime::reactor::create_reactor()
                        .map_err(|e| format!("Reactor error: {e}"))?;
                    let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
                        .with_reactor(reactor)
                        .build()
                        .map_err(|e| format!("Runtime error: {e}"))?;

                    let service = runtime
                        .block_on(AgentService::new(config))
                        .map_err(|e| format!("Agent re-init error: {e}"))?;
                    *guard = Some(service);
                    *version_guard = Some(settings_version);
                }
            }

            guard.take().unwrap()
        };

        let reactor = asupersync::runtime::reactor::create_reactor()
            .map_err(|e| format!("Reactor error: {e}"))?;
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .with_reactor(reactor)
            .build()
            .map_err(|e| format!("Runtime error: {e}"))?;

        let result = runtime.block_on(agent.prompt(message, on_event));

        {
            let mut guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
            *guard = Some(agent);
        }

        result.map_err(|e| format!("Agent error: {e}"))
    }

    pub async fn set_model(&self, provider: &str, model: &str) -> Result<(), String> {
        let mut agent = {
            let mut guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
            guard
                .take()
                .ok_or("Agent not initialized. Send a message first.")?
        };

        let reactor = asupersync::runtime::reactor::create_reactor()
            .map_err(|e| format!("Reactor error: {e}"))?;
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .with_reactor(reactor)
            .build()
            .map_err(|e| format!("Runtime error: {e}"))?;

        let result = runtime.block_on(agent.set_model(provider, model));

        {
            let mut guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
            *guard = Some(agent);
        }

        result.map_err(|e| format!("Set model error: {e}"))
    }

    pub fn get_model(&self) -> Result<(String, String), String> {
        let guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
        let agent = guard
            .as_ref()
            .ok_or("Agent not initialized. Send a message first.")?;
        Ok(agent.model())
    }

    pub fn get_settings(&self) -> Result<AgentSettingsDto, String> {
        self.settings.get_settings()
    }

    pub fn update_settings(
        &self,
        patch: AgentSettingsPatchDto,
    ) -> Result<AgentSettingsDto, String> {
        self.settings.update_settings(patch)
    }

    pub fn settings_catalog(&self) -> Result<AgentSettingsCatalogDto, String> {
        self.settings.catalog()
    }

    pub fn set_provider_api_key(&self, req: SetApiKeyRequest) -> Result<ProviderAuthDto, String> {
        self.settings.set_provider_api_key(req)
    }

    pub fn clear_provider_auth(&self, req: ProviderRequest) -> Result<ProviderAuthDto, String> {
        self.settings.clear_provider_auth(req)
    }

    pub fn set_provider_config(
        &self,
        req: SetProviderConfigRequest,
    ) -> Result<ProviderConfigDto, String> {
        self.settings.set_provider_config(req)
    }

    pub fn oauth_start(&self, req: ProviderRequest) -> Result<OauthStartResponse, String> {
        self.settings.start_oauth(req)
    }

    pub async fn oauth_status(
        &self,
        req: OauthStatusRequest,
    ) -> Result<OauthStatusResponse, String> {
        self.settings.oauth_status(req).await
    }

    pub fn oauth_cancel(&self, req: OauthStatusRequest) -> Result<OauthCancelResponse, String> {
        self.settings.cancel_oauth(req)
    }

    fn resolved_config(&self) -> Result<AgentServiceConfig, String> {
        let mut config = AgentServiceConfig::default();

        let mut current = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        while current.parent().is_some() {
            if current.join(".peekoo").is_dir() {
                config.working_directory = current.clone();
                break;
            }
            current = current.parent().unwrap().to_path_buf();
        }

        Ok(config)
    }
}
