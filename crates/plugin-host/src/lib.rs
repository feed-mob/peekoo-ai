use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::time::{Duration, timeout};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    TasksRead,
    TasksWrite,
    CalendarRead,
    CalendarWrite,
    ChatRespond,
    NotificationSend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub plugin_id: String,
    pub capabilities: Vec<Capability>,
}

impl PluginContext {
    pub fn has(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PluginError {
    #[error("capability denied")]
    CapabilityDenied,
    #[error("plugin execution timed out")]
    Timeout,
    #[error("plugin failed: {0}")]
    Failure(String),
}

#[async_trait]
pub trait Plugin: Send + Sync {
    async fn handle(&self, input: Value) -> Result<Value, PluginError>;
}

pub async fn execute_with_timeout(
    plugin: &dyn Plugin,
    input: Value,
    timeout_ms: u64,
) -> Result<Value, PluginError> {
    match timeout(Duration::from_millis(timeout_ms), plugin.handle(input)).await {
        Ok(result) => result,
        Err(_) => Err(PluginError::Timeout),
    }
}

pub fn require_capability(ctx: &PluginContext, capability: Capability) -> Result<(), PluginError> {
    if ctx.has(capability) {
        return Ok(());
    }
    Err(PluginError::CapabilityDenied)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OkPlugin;
    struct SlowPlugin;

    #[async_trait]
    impl Plugin for OkPlugin {
        async fn handle(&self, input: Value) -> Result<Value, PluginError> {
            Ok(input)
        }
    }

    #[async_trait]
    impl Plugin for SlowPlugin {
        async fn handle(&self, _input: Value) -> Result<Value, PluginError> {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(serde_json::json!({"ok": true}))
        }
    }

    #[tokio::test]
    async fn execute_returns_output_before_timeout() {
        let plugin = OkPlugin;
        let out = execute_with_timeout(&plugin, serde_json::json!({"k":"v"}), 20)
            .await
            .expect("plugin succeeds");
        assert_eq!(out["k"], "v");
    }

    #[tokio::test]
    async fn execute_fails_on_timeout() {
        let plugin = SlowPlugin;
        let out = execute_with_timeout(&plugin, serde_json::json!({}), 10).await;
        assert_eq!(out, Err(PluginError::Timeout));
    }

    #[test]
    fn require_capability_blocks_missing_permission() {
        let ctx = PluginContext {
            plugin_id: "p1".to_string(),
            capabilities: vec![Capability::TasksRead],
        };
        let result = require_capability(&ctx, Capability::TasksWrite);
        assert_eq!(result, Err(PluginError::CapabilityDenied));
    }
}
