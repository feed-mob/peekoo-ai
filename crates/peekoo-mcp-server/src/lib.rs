//! MCP Server for task management tools
//!
//! This crate provides an MCP (Model Context Protocol) server that exposes
//! task management tools for AI agents to use.

pub mod handler;

pub use handler::TaskMcpHandler;

use peekoo_productivity_domain::task::TaskService;
use rmcp::ServiceExt;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Start the MCP server on a TCP listener.
///
/// Returns the local address the server is listening on.
pub async fn start_tcp_server(
    task_service: Arc<dyn TaskService>,
    listener: TcpListener,
) -> Result<std::net::SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
    let local_addr = listener.local_addr()?;

    tracing::info!("MCP server starting on {}", local_addr);

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::info!("MCP client connected from {}", addr);
                    let handler = TaskMcpHandler::new(Arc::clone(&task_service));
                    tokio::spawn(async move {
                        let result = handler.serve(stream).await;
                        if let Err(e) = result {
                            tracing::error!("MCP server error for client {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept MCP connection: {}", e);
                }
            }
        }
    });

    Ok(local_addr)
}
