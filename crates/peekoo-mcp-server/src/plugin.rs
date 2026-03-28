//! MCP handler for plugin-provided tools.
//!
//! Exposes all tools from loaded plugins over MCP with namespaced names:
//! `plugin__{plugin_key}__{tool_name}`.

#[cfg(feature = "plugin-runtime")]
pub mod plugin_handler {
    use peekoo_plugin_host::PluginRegistry;
    use rmcp::{
        ErrorData as McpError,
        model::{
            CallToolRequestParams, CallToolResult, Content, ListToolsResult, ServerCapabilities,
            ServerInfo, Tool,
        },
        service::RequestContext,
        RoleServer, ServerHandler,
    };
    use std::sync::Arc;
    use serde_json::Map;

    const PLUGIN_PREFIX: &str = "plugin__";

    /// Builds the namespaced MCP tool name for a plugin tool.
    pub fn namespaced_name(plugin_key: &str, tool_name: &str) -> String {
        format!("{PLUGIN_PREFIX}{plugin_key}__{tool_name}")
    }

    /// Strips the `plugin__{key}__` prefix and returns `(plugin_key, tool_name)`.
    fn parse_namespaced_name(name: &str) -> Option<(String, String)> {
        let rest = name.strip_prefix(PLUGIN_PREFIX)?;
        let (plugin_key, tool_name) = rest.split_once("__")?;
        Some((plugin_key.to_string(), tool_name.to_string()))
    }

    #[derive(Clone)]
    pub struct PluginMcpHandler {
        registry: Arc<PluginRegistry>,
    }

    impl PluginMcpHandler {
        pub fn new(registry: Arc<PluginRegistry>) -> Self {
            Self { registry }
        }

        fn list_plugin_tools(&self) -> Vec<Tool> {
            self.registry
                .all_tool_definitions()
                .into_iter()
                .map(|(plugin_key, def)| {
                    let input_schema: Map<String, serde_json::Value> =
                        serde_json::from_str(&def.parameters).unwrap_or_default();
                    Tool::new(
                        namespaced_name(&plugin_key, &def.name),
                        def.description.clone(),
                        input_schema,
                    )
                })
                .collect()
        }
    }

    impl ServerHandler for PluginMcpHandler {
        fn get_info(&self) -> ServerInfo {
            ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
        }

        async fn list_tools(
            &self,
            _request: Option<rmcp::model::PaginatedRequestParams>,
            _context: RequestContext<RoleServer>,
        ) -> Result<ListToolsResult, McpError> {
            Ok(ListToolsResult {
                tools: self.list_plugin_tools(),
                ..Default::default()
            })
        }

        async fn call_tool(
            &self,
            request: CallToolRequestParams,
            _context: RequestContext<RoleServer>,
        ) -> Result<CallToolResult, McpError> {
            let (plugin_key, tool_name) =
                parse_namespaced_name(&request.name).ok_or_else(|| {
                    McpError::invalid_params(
                        format!("Unknown tool: {}", request.name),
                        None,
                    )
                })?;

            let args_json = request
                .arguments
                .map(|a| serde_json::to_string(&a).unwrap_or_else(|_| "{}".to_string()))
                .unwrap_or_else(|| "{}".to_string());

            // Plugin WASM calls are synchronous — run on a blocking thread.
            let registry = Arc::clone(&self.registry);
            let result = tokio::task::spawn_blocking(move || {
                registry.call_tool(&plugin_key, &tool_name, &args_json)
            })
            .await;

            match result {
                Ok(Ok(output)) => Ok(CallToolResult::success(vec![Content::text(output)])),
                Ok(Err(e)) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
                Err(join_err) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Plugin tool execution failed: {join_err}"
                ))])),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{namespaced_name, parse_namespaced_name};

        #[test]
        fn namespaced_name_format() {
            assert_eq!(
                namespaced_name("health-reminders", "get_status"),
                "plugin__health-reminders__get_status"
            );
        }

        #[test]
        fn parse_namespaced_name_roundtrip() {
            let name = namespaced_name("health-reminders", "get_status");
            let (key, tool) = parse_namespaced_name(&name).unwrap();
            assert_eq!(key, "health-reminders");
            assert_eq!(tool, "get_status");
        }

        #[test]
        fn parse_namespaced_name_rejects_non_prefixed() {
            assert!(parse_namespaced_name("task_comment").is_none());
            assert!(parse_namespaced_name("plugin__no_double_underscore").is_none());
        }
    }
}
