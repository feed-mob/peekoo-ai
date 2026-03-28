//! Shared HTTP MCP client adapter.
//!
//! Connects to a running MCP server over streamable HTTP, lists its tools,
//! and wraps each one as a [`pi::tools::Tool`] so the agent loop can invoke
//! them natively.
//!
//! # Usage
//!
//! ```rust,no_run
//! use peekoo_agent::mcp_client::connect_http_mcp_tools;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let (tools, _handle) = connect_http_mcp_tools("http://127.0.0.1:49152/mcp").await?;
//! // register `tools` with AgentService::register_native_tools(tools)
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use pi::error::Result as PiResult;
use pi::model::{ContentBlock, TextContent};
use pi::tools::{Tool, ToolOutput, ToolUpdate};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, Tool as McpTool},
    service::Peer,
    transport::StreamableHttpClientTransport,
};

type McpPeer = Peer<rmcp::service::RoleClient>;

// ── Public API ────────────────────────────────────────────────────────────────

/// Connect to an HTTP MCP server, list its tools, and return them as
/// [`pi::tools::Tool`] objects.
///
/// The returned [`McpClientHandle`] must be kept alive for as long as the
/// tools are in use — dropping it cancels the underlying connection.
pub async fn connect_http_mcp_tools(url: &str) -> Result<(Vec<Box<dyn Tool>>, McpClientHandle)> {
    ensure_rustls_provider();

    let transport = StreamableHttpClientTransport::from_uri(url.to_string());
    let client: rmcp::service::RunningService<rmcp::service::RoleClient, ()> =
        ().serve(transport).await?;

    let peer = client.peer().clone();
    let mcp_tools = peer.list_all_tools().await?;

    tracing::info!(
        url,
        tool_count = mcp_tools.len(),
        "Connected to MCP server"
    );

    let tools: Vec<Box<dyn Tool>> = mcp_tools
        .into_iter()
        .map(|t| Box::new(McpToolAdapter::new(peer.clone(), t)) as Box<dyn Tool>)
        .collect();

    let handle = McpClientHandle {
        shutdown: Box::new(move || {
            client.cancellation_token().cancel();
        }),
    };

    Ok((tools, handle))
}

// ── Handle ────────────────────────────────────────────────────────────────────

/// Keeps the MCP client connection alive.
///
/// Drop this to cancel the connection. Keep it alive for as long as the
/// tools returned by [`connect_http_mcp_tools`] are registered with an agent.
pub struct McpClientHandle {
    shutdown: Box<dyn Fn() + Send + Sync>,
}

impl Drop for McpClientHandle {
    fn drop(&mut self) {
        (self.shutdown)();
    }
}

// ── Adapter ───────────────────────────────────────────────────────────────────

/// Wraps a single MCP tool as a [`pi::tools::Tool`].
struct McpToolAdapter {
    peer: Arc<McpPeer>,
    tool: McpTool,
}

impl McpToolAdapter {
    fn new(peer: McpPeer, tool: McpTool) -> Self {
        Self {
            peer: Arc::new(peer),
            tool,
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        self.tool.name.as_ref()
    }

    fn label(&self) -> &str {
        self.tool.name.as_ref()
    }

    fn description(&self) -> &str {
        self.tool.description.as_deref().unwrap_or("MCP tool")
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(&self.tool.input_schema)
            .unwrap_or_else(|_| serde_json::json!({"type": "object", "properties": {}}))
    }

    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> PiResult<ToolOutput> {
        let arguments = match input {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };

        let request =
            CallToolRequestParams::new(self.tool.name.clone()).with_arguments(arguments);

        let result = self
            .peer
            .call_tool(request)
            .await
            .map_err(|e| pi::error::Error::Tool {
                tool: self.tool.name.to_string(),
                message: e.to_string(),
            })?;

        let mut chunks: Vec<String> = result
            .content
            .iter()
            .filter_map(|item| item.raw.as_text().map(|t| t.text.clone()))
            .collect();

        if chunks.is_empty() {
            if let Some(structured) = result.structured_content {
                chunks.push(serde_json::to_string_pretty(&structured).unwrap_or_default());
            } else {
                chunks.push(serde_json::to_string(&result).unwrap_or_default());
            }
        }

        Ok(ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(chunks.join("\n\n")))],
            details: None,
            is_error: result.is_error.unwrap_or(false),
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn ensure_rustls_provider() {
    static RUSTLS_PROVIDER: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    RUSTLS_PROVIDER.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
