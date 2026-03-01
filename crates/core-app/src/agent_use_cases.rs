//! Agent use cases — application-level orchestration for the AI agent.
//!
//! Bridges the agent service with the event bus, publishing domain events
//! for prompt lifecycle so the UI (Tauri) layer can react.

use peekoo_agent::config::AgentServiceConfig;
use peekoo_agent::service::AgentService;
use peekoo_agent::{AgentEvent, PiResult};
use peekoo_event_bus::{EventBus, EventEnvelope};

/// Application-level agent orchestration.
///
/// Wraps [`AgentService`] and publishes domain events through the [`EventBus`].
pub struct AgentUseCases {
    bus: EventBus,
    agent: AgentService,
}

impl AgentUseCases {
    /// Create a new agent use case layer.
    pub async fn create(bus: EventBus, config: AgentServiceConfig) -> PiResult<Self> {
        let agent = AgentService::new(config).await?;
        Ok(Self { bus, agent })
    }

    /// Send a chat message and return the assistant's reply text.
    ///
    /// Publishes `v1.agent.prompt_started` and `v1.agent.response_received`
    /// events on the bus.
    pub async fn chat(&mut self, message: &str) -> PiResult<String> {
        let _ = self.bus.publish(EventEnvelope {
            trace_id: format!("agent-{}", uuid_v4()),
            event_type: "v1.agent.prompt_started".to_string(),
            schema_version: "v1".to_string(),
            payload: serde_json::json!({ "message": message }),
        });

        let reply = self.agent.prompt(message, |_event| {}).await?;

        let _ = self.bus.publish(EventEnvelope {
            trace_id: format!("agent-{}", uuid_v4()),
            event_type: "v1.agent.response_received".to_string(),
            schema_version: "v1".to_string(),
            payload: serde_json::json!({ "reply": &reply }),
        });

        Ok(reply)
    }

    /// Send a chat message with a streaming event callback.
    pub async fn chat_streaming(
        &mut self,
        message: &str,
        on_event: impl Fn(AgentEvent) + Send + Sync + 'static,
    ) -> PiResult<String> {
        self.agent.prompt(message, on_event).await
    }

    /// Switch the active LLM provider and model.
    pub async fn switch_model(&mut self, provider: &str, model: &str) -> PiResult<()> {
        self.agent.set_model(provider, model).await?;

        let _ = self.bus.publish(EventEnvelope {
            trace_id: format!("agent-{}", uuid_v4()),
            event_type: "v1.agent.model_switched".to_string(),
            schema_version: "v1".to_string(),
            payload: serde_json::json!({
                "provider": provider,
                "model": model,
            }),
        });

        Ok(())
    }

    /// Return the currently active `(provider, model)` pair.
    pub fn current_model(&self) -> (String, String) {
        self.agent.model()
    }

    /// Access the underlying agent service for advanced operations.
    pub fn agent(&self) -> &AgentService {
        &self.agent
    }

    /// Mutable access to the underlying agent service.
    pub fn agent_mut(&mut self) -> &mut AgentService {
        &mut self.agent
    }
}

fn uuid_v4() -> String {
    // Simple timestamp-based pseudo-unique ID to avoid adding uuid dep.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}-{:x}", now.as_secs(), now.subsec_nanos())
}
