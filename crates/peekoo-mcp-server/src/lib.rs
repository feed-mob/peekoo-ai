//! MCP Server for task management tools.
//!
//! This crate exposes task tools over RMCP's streamable HTTP transport.

pub mod handler;

pub use handler::TaskMcpHandler;

use axum::Router;
use peekoo_productivity_domain::task::TaskService;
use rmcp::transport::{
    StreamableHttpServerConfig,
    streamable_http_server::{session::local::LocalSessionManager, tower::StreamableHttpService},
};
use std::sync::Arc;
use tokio::net::TcpListener;

pub const MCP_PATH: &str = "/mcp";

pub fn mcp_url_for(addr: std::net::SocketAddr) -> String {
    format!("http://{}{}", addr, MCP_PATH)
}

#[cfg(test)]
fn ensure_rustls_provider() {
    static RUSTLS_PROVIDER: std::sync::OnceLock<()> = std::sync::OnceLock::new();

    RUSTLS_PROVIDER.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Start the MCP server on a TCP listener.
///
/// Returns the local address the HTTP server is listening on. The MCP endpoint is
/// available at `http://{addr}/mcp`.
pub async fn start_tcp_server(
    task_service: Arc<dyn TaskService>,
    listener: TcpListener,
) -> Result<std::net::SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
    let local_addr = listener.local_addr()?;

    tracing::info!(
        "MCP streamable HTTP server starting on {}",
        mcp_url_for(local_addr)
    );

    let mcp_service: StreamableHttpService<TaskMcpHandler, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(TaskMcpHandler::new(Arc::clone(&task_service))),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig::default(),
        );

    let app = Router::new().nest_service(MCP_PATH, mcp_service);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("MCP HTTP server exited with error: {}", e);
        }
    });

    Ok(local_addr)
}

#[cfg(test)]
mod tests {
    use super::{mcp_url_for, start_tcp_server};
    use peekoo_productivity_domain::task::NoopTaskService;
    use rmcp::{ServiceExt, transport::StreamableHttpClientTransport};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn http_server_keeps_connection_alive_for_list_tools() {
        super::ensure_rustls_provider();

        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind listener");
        let addr = start_tcp_server(Arc::new(NoopTaskService), listener)
            .await
            .expect("start server");

        let transport = StreamableHttpClientTransport::from_uri(mcp_url_for(addr));
        let client: rmcp::service::RunningService<rmcp::service::RoleClient, ()> =
            ().serve(transport).await.expect("complete handshake");

        let tools = client
            .list_tools(Default::default())
            .await
            .expect("list tools without transport closing");

        let tool_names: Vec<String> = tools
            .tools
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect();

        assert!(tool_names.iter().any(|name| name == "task_comment"));
        assert!(tool_names.iter().any(|name| name == "update_task_labels"));
        assert!(tool_names.iter().any(|name| name == "update_task_status"));
    }
}
