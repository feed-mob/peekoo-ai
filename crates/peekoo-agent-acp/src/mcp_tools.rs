use agent_client_protocol::{McpServer, McpServerHttp, McpServerStdio};
use anyhow::{Result as AnyResult, anyhow};
use async_trait::async_trait;
use peekoo_agent::AgentEvent;
use pi::error::Result;
use pi::model::{ContentBlock, TextContent};
use pi::tools::{Tool, ToolOutput, ToolUpdate};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, Tool as McpTool},
    service::Peer,
    transport::{StreamableHttpClientTransport, TokioChildProcess},
};
use tokio::process::Command;

type McpPeer = Peer<rmcp::service::RoleClient>;

pub async fn connect_task_mcp_tools(
    task_id: &str,
    servers: &[McpServer],
) -> AnyResult<(Vec<Box<dyn Tool>>, Vec<McpClientHandle>)> {
    let mut all_tools: Vec<Box<dyn Tool>> = Vec::new();
    let mut handles = Vec::new();

    for server in servers {
        match connect_server(server).await {
            Ok(handle) => {
                let peer = handle.peer.clone();
                let tools = peer.list_all_tools().await?;
                tracing::info!(
                    "Connected MCP server '{}' with {} tools",
                    handle.name,
                    tools.len()
                );

                for tool in tools {
                    all_tools.push(Box::new(McpToolAdapter::new(
                        task_id.to_string(),
                        peer.clone(),
                        tool,
                    )) as Box<dyn Tool>);
                }

                handles.push(handle);
            }
            Err(error) => {
                tracing::error!("Failed to connect MCP server {:?}: {}", server, error);
            }
        }
    }

    Ok((all_tools, handles))
}

pub fn summarize_agent_event(event: &AgentEvent) -> Option<String> {
    match event {
        AgentEvent::ToolExecutionStart {
            tool_name, args, ..
        } => Some(format!(
            "Running tool `{tool_name}` with args `{}`...",
            args
        )),
        AgentEvent::ToolExecutionEnd {
            tool_name,
            result,
            is_error,
            ..
        } => {
            let details = tool_output_summary(result);
            if *is_error {
                Some(format!("Tool `{tool_name}` reported an error: {details}"))
            } else {
                Some(format!("Tool `{tool_name}` completed: {details}"))
            }
        }
        _ => None,
    }
}

fn tool_output_summary(output: &pi::tools::ToolOutput) -> String {
    let mut parts = Vec::new();
    for block in &output.content {
        if let ContentBlock::Text(text) = block {
            let trimmed = text.text.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.to_string());
            }
        }
    }

    if parts.is_empty() {
        if output.is_error {
            "tool failed without details".to_string()
        } else {
            "ok".to_string()
        }
    } else {
        parts.join(" | ")
    }
}

pub struct McpClientHandle {
    pub name: String,
    peer: McpPeer,
    shutdown: Box<dyn Fn() + Send + Sync>,
}

impl Drop for McpClientHandle {
    fn drop(&mut self) {
        (self.shutdown)();
    }
}

async fn connect_server(server: &McpServer) -> AnyResult<McpClientHandle> {
    match server {
        McpServer::Http(http) => connect_http_server(http).await,
        McpServer::Stdio(stdio) => connect_stdio_server(stdio).await,
        McpServer::Sse(sse) => Err(anyhow!(
            "SSE MCP transport is not supported for task execution: {}",
            sse.url
        )),
        _ => Err(anyhow!("Unsupported MCP server transport")),
    }
}

async fn connect_http_server(server: &McpServerHttp) -> AnyResult<McpClientHandle> {
    super::agent::ensure_rustls_provider();

    let transport = StreamableHttpClientTransport::from_uri(server.url.clone());
    let client: rmcp::service::RunningService<rmcp::service::RoleClient, ()> =
        ().serve(transport).await?;

    let peer = client.peer().clone();
    let name = server.name.clone();

    Ok(McpClientHandle {
        name,
        peer,
        shutdown: Box::new(move || {
            let token = client.cancellation_token();
            token.cancel();
        }),
    })
}

async fn connect_stdio_server(server: &McpServerStdio) -> AnyResult<McpClientHandle> {
    let mut command = Command::new(&server.command);
    command.args(&server.args);

    for env_var in &server.env {
        command.env(&env_var.name, &env_var.value);
    }

    let child = TokioChildProcess::new(command)?;
    let client: rmcp::service::RunningService<rmcp::service::RoleClient, ()> =
        ().serve(child).await?;

    let peer = client.peer().clone();
    let name = server.name.clone();

    Ok(McpClientHandle {
        name,
        peer,
        shutdown: Box::new(move || {
            let token = client.cancellation_token();
            token.cancel();
        }),
    })
}

struct McpToolAdapter {
    task_id: String,
    peer: McpPeer,
    tool: McpTool,
}

impl McpToolAdapter {
    fn new(task_id: String, peer: McpPeer, tool: McpTool) -> Self {
        Self {
            task_id,
            peer,
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
        self.tool
            .description
            .as_deref()
            .unwrap_or("MCP bridged tool")
    }

    fn parameters(&self) -> serde_json::Value {
        match self.tool.name.as_ref() {
            "task_comment" => serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Comment text to add to the current task"
                    }
                },
                "required": ["text"]
            }),
            "update_task_status" => serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["pending", "in_progress", "done", "cancelled"],
                        "description": "New status for the current task"
                    }
                },
                "required": ["status"]
            }),
            "update_task_labels" => serde_json::json!({
                "type": "object",
                "properties": {
                    "add_labels": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Labels to add to the current task"
                    },
                    "remove_labels": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Labels to remove from the current task"
                    }
                }
            }),
            _ => serde_json::to_value(&self.tool.input_schema)
                .unwrap_or_else(|_| serde_json::json!({})),
        }
    }

    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let arguments = match (self.tool.name.as_ref(), input) {
            ("task_comment", serde_json::Value::Object(mut arguments)) => {
                arguments.insert(
                    "task_id".to_string(),
                    serde_json::Value::String(self.task_id.clone()),
                );
                arguments
            }
            ("update_task_status", serde_json::Value::Object(mut arguments)) => {
                arguments.insert(
                    "task_id".to_string(),
                    serde_json::Value::String(self.task_id.clone()),
                );
                arguments
            }
            ("update_task_labels", serde_json::Value::Object(mut arguments)) => {
                arguments.insert(
                    "task_id".to_string(),
                    serde_json::Value::String(self.task_id.clone()),
                );
                arguments
            }
            (_, serde_json::Value::Object(arguments)) => arguments,
            _ => serde_json::Map::new(),
        };

        let request = CallToolRequestParams::new(self.tool.name.clone()).with_arguments(arguments);

        let result =
            self.peer
                .call_tool(request)
                .await
                .map_err(|error| pi::error::Error::Tool {
                    tool: self.tool.name.to_string(),
                    message: error.to_string(),
                })?;

        let mut chunks = Vec::new();
        for item in &result.content {
            if let Some(text) = item.raw.as_text() {
                chunks.push(text.text.clone());
            }
        }

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
}
