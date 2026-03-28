//! Bridge between ACP agents and peekoo's MCP server
//!
//! For agents that don't support MCP natively, this bridge:
//! 1. Intercepts tool use requests from the agent
//! 2. Executes tools through peekoo's MCP server
//! 3. Returns results to the agent

use crate::backend::ContentBlock;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool result from execution
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

/// MCP tool information for discovery
#[derive(Debug, Clone)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP bridge for tool execution
pub struct McpBridge {
    /// MCP server URL
    mcp_url: String,
    /// Cached tool list
    tools: Arc<RwLock<Vec<McpToolInfo>>>,
    /// Tool result cache (tool_call_id -> result)
    result_cache: Arc<RwLock<HashMap<String, ToolResult>>>,
    /// Connection status
    connected: bool,
}

impl McpBridge {
    /// Create a new MCP bridge with the given MCP server URL
    pub fn new(mcp_url: String) -> Self {
        Self {
            mcp_url,
            tools: Arc::new(RwLock::new(Vec::new())),
            result_cache: Arc::new(RwLock::new(HashMap::new())),
            connected: false,
        }
    }

    /// Check if bridge has an active connection
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Connect to the MCP server
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        // For now, connection is handled lazily
        // In production, this would establish the actual MCP connection
        self.connected = true;
        tracing::info!("MCP bridge connected to: {}", self.mcp_url);
        Ok(())
    }

    /// Execute a tool call through MCP
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        // In production, this would call the actual MCP server
        // For now, return a mock result for testing

        tracing::info!(
            "Executing tool '{}' with arguments: {}",
            tool_name,
            arguments
        );

        // Mock execution - in production, this would use rmcp client
        let content = format!("Tool '{}' executed with args: {}", tool_name, arguments);

        Ok(ToolResult {
            content,
            is_error: false,
        })
    }

    /// Handle tool use from an ACP agent
    ///
    /// When an ACP agent requests a tool:
    /// 1. Parse the tool name and arguments
    /// 2. Execute via MCP
    /// 3. Return the result formatted for the agent
    pub async fn handle_tool_use(
        &self,
        tool_use_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> anyhow::Result<ContentBlock> {
        // Execute the tool
        let result = match self.execute_tool(tool_name, arguments).await {
            Ok(r) => r,
            Err(e) => ToolResult {
                content: format!("Tool execution error: {}", e),
                is_error: true,
            },
        };

        // Cache the result
        {
            let mut cache = self.result_cache.write().await;
            cache.insert(tool_use_id.to_string(), result.clone());
        }

        // Return as ContentBlock::ToolResult
        Ok(ContentBlock::ToolResult {
            tool_use_id: tool_use_id.to_string(),
            content: result.content,
            is_error: result.is_error,
        })
    }

    /// Get available tools from MCP server
    pub async fn list_tools(&self) -> anyhow::Result<Vec<McpToolInfo>> {
        // Return cached tools if available
        {
            let tools = self.tools.read().await;
            if !tools.is_empty() {
                return Ok(tools.clone());
            }
        }

        // In production, this would fetch from MCP server
        // For now, return mock tools for testing
        let mock_tools = vec![
            McpToolInfo {
                name: "read_file".to_string(),
                description: "Read the contents of a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path to the file" }
                    },
                    "required": ["path"]
                }),
            },
            McpToolInfo {
                name: "write_file".to_string(),
                description: "Write content to a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path to the file" },
                        "content": { "type": "string", "description": "Content to write" }
                    },
                    "required": ["path", "content"]
                }),
            },
        ];

        // Cache the tools
        {
            let mut tools = self.tools.write().await;
            *tools = mock_tools.clone();
        }

        Ok(mock_tools)
    }

    /// Convert MCP tools to format suitable for system prompt
    ///
    /// Some ACP agents need tools described in their system prompt
    pub async fn generate_tools_prompt(&self) -> anyhow::Result<String> {
        let tools = self.list_tools().await?;

        if tools.is_empty() {
            return Ok("No tools available.".to_string());
        }

        let mut prompt = String::from("Available tools:\n\n");

        for tool in tools {
            prompt.push_str(&format!("### {}\n", tool.name));
            prompt.push_str(&format!("Description: {}\n", tool.description));
            prompt.push_str(&format!(
                "Parameters: {}\n",
                serde_json::to_string_pretty(&tool.input_schema)?
            ));
            prompt.push_str("\n");
        }

        prompt.push_str("\nWhen you need to use a tool, respond with:\n");
        prompt.push_str("```tool\n");
        prompt.push_str("{\"tool\": \"tool_name\", \"arguments\": {...}}\n");
        prompt.push_str("```\n");

        Ok(prompt)
    }

    /// Get a cached tool result
    pub async fn get_cached_result(&self, tool_call_id: &str) -> Option<ToolResult> {
        let cache = self.result_cache.read().await;
        cache.get(tool_call_id).cloned()
    }

    /// Clear the result cache
    pub async fn clear_cache(&self) {
        let mut cache = self.result_cache.write().await;
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_bridge_new() {
        let bridge = McpBridge::new("http://localhost:3000/mcp".to_string());
        assert!(!bridge.is_connected());
        assert_eq!(bridge.mcp_url, "http://localhost:3000/mcp");
    }

    #[tokio::test]
    async fn test_mcp_bridge_connect() {
        let mut bridge = McpBridge::new("http://localhost:3000/mcp".to_string());
        let result = bridge.connect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_tool() {
        let bridge = McpBridge::new("http://localhost:3000/mcp".to_string());

        let result = bridge
            .execute_tool("read_file", serde_json::json!({ "path": "/tmp/test.txt" }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("read_file"));
        assert!(result.content.contains("/tmp/test.txt"));
    }

    #[tokio::test]
    async fn test_handle_tool_use() {
        let bridge = McpBridge::new("http://localhost:3000/mcp".to_string());

        let result = bridge
            .handle_tool_use(
                "call_123",
                "read_file",
                serde_json::json!({ "path": "/tmp/test.txt" }),
            )
            .await
            .unwrap();

        match result {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "call_123");
                assert!(!is_error);
                assert!(content.contains("read_file"));
            }
            _ => panic!("Expected ToolResult content block"),
        }

        // Check it was cached
        let cached = bridge.get_cached_result("call_123").await;
        assert!(cached.is_some());
    }

    #[tokio::test]
    async fn test_list_tools() {
        let bridge = McpBridge::new("http://localhost:3000/mcp".to_string());

        let tools = bridge.list_tools().await.unwrap();

        assert!(!tools.is_empty());

        // Should have read_file and write_file
        let has_read = tools.iter().any(|t| t.name == "read_file");
        let has_write = tools.iter().any(|t| t.name == "write_file");
        assert!(has_read);
        assert!(has_write);
    }

    #[tokio::test]
    async fn test_list_tools_cached() {
        let bridge = McpBridge::new("http://localhost:3000/mcp".to_string());

        // First call populates cache
        let tools1 = bridge.list_tools().await.unwrap();

        // Second call should use cache
        let tools2 = bridge.list_tools().await.unwrap();

        assert_eq!(tools1.len(), tools2.len());
    }

    #[tokio::test]
    async fn test_generate_tools_prompt() {
        let bridge = McpBridge::new("http://localhost:3000/mcp".to_string());

        let prompt = bridge.generate_tools_prompt().await.unwrap();

        assert!(prompt.contains("Available tools:"));
        assert!(prompt.contains("read_file"));
        assert!(prompt.contains("write_file"));
        assert!(prompt.contains("When you need to use a tool"));
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let bridge = McpBridge::new("http://localhost:3000/mcp".to_string());

        // Add something to cache
        bridge
            .handle_tool_use("call_1", "test", serde_json::json!({}))
            .await
            .unwrap();

        // Verify it's cached
        assert!(bridge.get_cached_result("call_1").await.is_some());

        // Clear cache
        bridge.clear_cache().await;

        // Verify it's gone
        assert!(bridge.get_cached_result("call_1").await.is_none());
    }

    #[tokio::test]
    async fn test_tool_with_error() {
        // This test would need a mock that returns errors
        // For now, we just test the success path
        let bridge = McpBridge::new("http://localhost:3000/mcp".to_string());

        let result = bridge
            .execute_tool("unknown_tool", serde_json::json!({}))
            .await
            .unwrap();

        // Mock returns success for all tools
        assert!(!result.is_error);
    }
}
