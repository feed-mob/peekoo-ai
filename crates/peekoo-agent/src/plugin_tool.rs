//! Plugin tool integration — bridges plugin-provided tools into the pi agent
//! tool registry via dependency inversion.
//!
//! The [`PluginToolProvider`] trait defines a minimal contract that decouples
//! this crate from the concrete plugin host implementation. The orchestration
//! layer (`peekoo-agent-app`) provides the concrete implementation.
//!
//! [`PluginToolAdapter`] wraps a single plugin tool behind pi's [`Tool`] trait
//! so the LLM can invoke it natively during the agent loop.

use std::sync::Arc;

use async_trait::async_trait;
use pi::error::Result;
use pi::model::{ContentBlock, TextContent};
use pi::tools::{Tool, ToolOutput, ToolUpdate};

// ============================================================================
// Provider trait (dependency-inverted contract)
// ============================================================================

/// Describes a single plugin tool in a provider-agnostic way.
#[derive(Debug, Clone)]
pub struct PluginToolSpec {
    /// Tool name as declared by the plugin (e.g. `health_get_status`).
    pub name: String,
    /// Human-readable description shown to the LLM.
    pub description: String,
    /// JSON Schema describing the tool's parameters.
    pub parameters_schema: serde_json::Value,
    /// Key of the plugin that owns this tool.
    pub plugin_key: String,
}

/// Minimal trait for routing tool calls to the plugin runtime.
///
/// Implemented by the app layer (e.g. for [`PluginToolBridge`]) so that
/// `peekoo-agent` never depends on `peekoo-plugin-host` directly.
pub trait PluginToolProvider: Send + Sync {
    /// Return specs for every tool exposed by loaded plugins.
    fn tool_specs(&self) -> Vec<PluginToolSpec>;

    /// Execute a plugin tool owned by `plugin_key`.
    ///
    /// Both `plugin_key` and `tool_name` are required so the provider can
    /// dispatch to the correct plugin even when multiple plugins export a
    /// tool with the same name.
    ///
    /// `args_json` is the JSON-serialised arguments object from the LLM.
    /// Returns the tool's JSON result string on success.
    fn call_tool(
        &self,
        plugin_key: &str,
        tool_name: &str,
        args_json: &str,
    ) -> std::result::Result<String, String>;
}

// ============================================================================
// Adapter (implements pi's Tool trait)
// ============================================================================

/// Wraps a single plugin tool so it satisfies pi's [`Tool`] trait.
///
/// Tool names are namespaced as `plugin__{plugin_key}__{tool_name}` to avoid
/// collisions with the built-in tools (read, bash, edit, write, grep, find, ls).
pub struct PluginToolAdapter {
    /// Namespaced tool name: `plugin__{plugin_key}__{tool_name}`.
    namespaced_name: String,
    /// Original tool name as declared in the plugin manifest.
    original_name: String,
    /// Key of the plugin that owns this tool (used for dispatch).
    plugin_key: String,
    description: String,
    parameters_schema: serde_json::Value,
    provider: Arc<dyn PluginToolProvider>,
}

impl PluginToolAdapter {
    /// Build a namespaced tool name: `plugin__{plugin_key}__{tool_name}`.
    fn namespaced_name(plugin_key: &str, tool_name: &str) -> String {
        format!("plugin__{plugin_key}__{tool_name}")
    }

    /// Create an adapter for a single plugin tool.
    pub fn new(spec: PluginToolSpec, provider: Arc<dyn PluginToolProvider>) -> Self {
        let namespaced_name = Self::namespaced_name(&spec.plugin_key, &spec.name);
        Self {
            namespaced_name,
            original_name: spec.name,
            plugin_key: spec.plugin_key,
            description: spec.description,
            parameters_schema: spec.parameters_schema,
            provider,
        }
    }

    /// Create adapters for every tool exposed by the provider.
    pub fn from_provider(provider: Arc<dyn PluginToolProvider>) -> Vec<Box<dyn Tool>> {
        provider
            .tool_specs()
            .into_iter()
            .map(|spec| Box::new(Self::new(spec, Arc::clone(&provider))) as Box<dyn Tool>)
            .collect()
    }
}

#[async_trait]
impl Tool for PluginToolAdapter {
    fn name(&self) -> &str {
        &self.namespaced_name
    }

    fn label(&self) -> &str {
        &self.namespaced_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> serde_json::Value {
        self.parameters_schema.clone()
    }

    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let args_json = serde_json::to_string(&input).unwrap_or_else(|_| "{}".to_string());
        let plugin_key = self.plugin_key.clone();
        let original_name = self.original_name.clone();
        let provider = Arc::clone(&self.provider);

        // Plugin tools execute WASM synchronously via Extism. Run on a blocking
        // thread to avoid stalling the async runtime.
        let result = tokio::task::spawn_blocking(move || {
            provider.call_tool(&plugin_key, &original_name, &args_json)
        })
        .await;

        match result {
            Ok(Ok(output_json)) => Ok(ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(output_json))],
                details: None,
                is_error: false,
            }),
            Ok(Err(err)) => Ok(ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(format!(
                    "Plugin tool error: {err}"
                )))],
                details: None,
                is_error: true,
            }),
            Err(join_err) => Ok(ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(format!(
                    "Plugin tool execution failed: {join_err}"
                )))],
                details: None,
                is_error: true,
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        // Conservative: plugin tools may have side effects.
        false
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    /// Stub provider for testing.
    struct StubProvider {
        specs: Vec<PluginToolSpec>,
        result: std::result::Result<String, String>,
        /// Records (plugin_key, tool_name) from the last call for assertions.
        last_call: Mutex<Option<(String, String)>>,
    }

    impl StubProvider {
        fn new(specs: Vec<PluginToolSpec>, result: std::result::Result<String, String>) -> Self {
            Self {
                specs,
                result,
                last_call: Mutex::new(None),
            }
        }
    }

    impl PluginToolProvider for StubProvider {
        fn tool_specs(&self) -> Vec<PluginToolSpec> {
            self.specs.clone()
        }

        fn call_tool(
            &self,
            plugin_key: &str,
            tool_name: &str,
            _args_json: &str,
        ) -> std::result::Result<String, String> {
            *self.last_call.lock().unwrap() = Some((plugin_key.to_string(), tool_name.to_string()));
            self.result.clone()
        }
    }

    fn sample_spec() -> PluginToolSpec {
        PluginToolSpec {
            name: "get_status".to_string(),
            description: "Get the current status".to_string(),
            parameters_schema: serde_json::json!({"type": "object", "properties": {}}),
            plugin_key: "health-reminders".to_string(),
        }
    }

    #[test]
    fn namespaced_name_format() {
        let name = PluginToolAdapter::namespaced_name("health-reminders", "get_status");
        assert_eq!(name, "plugin__health-reminders__get_status");
    }

    #[test]
    fn adapter_metadata() {
        let provider = Arc::new(StubProvider::new(vec![sample_spec()], Ok("{}".to_string())));
        let adapter = PluginToolAdapter::new(sample_spec(), provider);

        assert_eq!(adapter.name(), "plugin__health-reminders__get_status");
        assert_eq!(adapter.label(), "plugin__health-reminders__get_status");
        assert_eq!(adapter.description(), "Get the current status");
        assert!(!adapter.is_read_only());
    }

    #[test]
    fn from_provider_creates_adapters_for_all_specs() {
        let provider = Arc::new(StubProvider::new(
            vec![
                PluginToolSpec {
                    name: "tool_a".to_string(),
                    description: "A".to_string(),
                    parameters_schema: serde_json::json!({}),
                    plugin_key: "plug".to_string(),
                },
                PluginToolSpec {
                    name: "tool_b".to_string(),
                    description: "B".to_string(),
                    parameters_schema: serde_json::json!({}),
                    plugin_key: "plug".to_string(),
                },
            ],
            Ok("{}".to_string()),
        ));

        let tools = PluginToolAdapter::from_provider(provider);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name(), "plugin__plug__tool_a");
        assert_eq!(tools[1].name(), "plugin__plug__tool_b");
    }

    #[test]
    fn from_provider_empty_when_no_specs() {
        let provider = Arc::new(StubProvider::new(vec![], Ok("{}".to_string())));

        let tools = PluginToolAdapter::from_provider(provider);
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn execute_dispatches_with_plugin_key() {
        let concrete = Arc::new(StubProvider::new(
            vec![sample_spec()],
            Ok(r#"{"healthy":true}"#.to_string()),
        ));
        let provider: Arc<dyn PluginToolProvider> = Arc::clone(&concrete) as _;
        let adapter = PluginToolAdapter::new(sample_spec(), provider);

        let output = adapter
            .execute("call-1", serde_json::json!({}), None)
            .await
            .expect("execute should succeed");

        assert!(!output.is_error);
        assert_eq!(output.content.len(), 1);
        match &output.content[0] {
            ContentBlock::Text(t) => assert_eq!(t.text, r#"{"healthy":true}"#),
            other => panic!("Expected Text block, got: {other:?}"),
        }

        // Verify plugin_key was passed through to the provider.
        let last = concrete.last_call.lock().unwrap().clone().unwrap();
        assert_eq!(last.0, "health-reminders", "plugin_key must be forwarded");
        assert_eq!(last.1, "get_status", "tool_name must be the original name");
    }

    #[tokio::test]
    async fn execute_tool_error() {
        let provider = Arc::new(StubProvider::new(
            vec![sample_spec()],
            Err("plugin crashed".to_string()),
        ));
        let adapter = PluginToolAdapter::new(sample_spec(), provider);

        let output = adapter
            .execute("call-2", serde_json::json!({}), None)
            .await
            .expect("execute should succeed even on tool error");

        assert!(output.is_error);
        match &output.content[0] {
            ContentBlock::Text(t) => assert!(t.text.contains("plugin crashed")),
            other => panic!("Expected Text block, got: {other:?}"),
        }
    }
}
