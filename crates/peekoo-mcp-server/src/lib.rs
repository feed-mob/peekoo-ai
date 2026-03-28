//! MCP Server for task management and plugin tools.
//!
//! This crate exposes Peekoo-owned tools over RMCP's streamable HTTP transport.
//! Task tools are always available. Plugin tools are available when the
//! `plugin-runtime` feature is enabled and a [`PluginRegistry`] is provided.

pub mod handler;
pub mod plugin;

pub use handler::TaskMcpHandler;

use axum::Router;
use peekoo_app_settings::AppSettingsService;
use peekoo_pomodoro_app::PomodoroAppService;
use peekoo_task_app::TaskService;
use rmcp::transport::{
    StreamableHttpServerConfig,
    streamable_http_server::{session::local::LocalSessionManager, tower::StreamableHttpService},
};
use std::sync::Arc;
use tokio::net::TcpListener;

#[cfg(feature = "plugin-runtime")]
use peekoo_plugin_host::PluginRegistry;

pub const MCP_PATH: &str = "/mcp";

pub fn mcp_url_for(addr: std::net::SocketAddr) -> String {
    format!("http://{}{}", addr, MCP_PATH)
}

pub fn mcp_plugins_url_for(addr: std::net::SocketAddr) -> String {
    format!("http://{}{}/plugins", addr, MCP_PATH)
}

pub fn ensure_rustls_provider() {
    static RUSTLS_PROVIDER: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    RUSTLS_PROVIDER.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Start the MCP server on a TCP listener.
///
/// - `task_service` — provides all task tools.
/// - `pomodoro_service` — provides pomodoro timer tools.
/// - `app_settings_service` — provides app settings tools.
/// - `plugin_registry` — when provided (requires `plugin-runtime` feature),
///   plugin tools are also exposed under `plugin__{key}__{name}` names.
///
/// Returns the local address the HTTP server is listening on. The MCP endpoint
/// is available at `http://{addr}/mcp`.
pub async fn start_tcp_server(
    task_service: Arc<dyn TaskService>,
    pomodoro_service: Arc<PomodoroAppService>,
    app_settings_service: Arc<AppSettingsService>,
    #[cfg(feature = "plugin-runtime")] plugin_registry: Option<Arc<PluginRegistry>>,
    #[cfg(not(feature = "plugin-runtime"))] _plugin_registry: Option<()>,
    listener: TcpListener,
) -> Result<std::net::SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
    let local_addr = listener.local_addr()?;

    tracing::info!(
        "MCP streamable HTTP server starting on {}",
        mcp_url_for(local_addr)
    );

    let app = build_router(
        task_service,
        pomodoro_service,
        app_settings_service,
        {
            #[cfg(feature = "plugin-runtime")]
            { plugin_registry }
            #[cfg(not(feature = "plugin-runtime"))]
            { None::<()> }
        },
    );

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("MCP HTTP server exited with error: {}", e);
        }
    });

    Ok(local_addr)
}

fn build_router(
    task_service: Arc<dyn TaskService>,
    pomodoro_service: Arc<PomodoroAppService>,
    app_settings_service: Arc<AppSettingsService>,
    #[cfg(feature = "plugin-runtime")] plugin_registry: Option<Arc<PluginRegistry>>,
    #[cfg(not(feature = "plugin-runtime"))] _plugin_registry: Option<()>,
) -> Router {
    let task_service_clone = Arc::clone(&task_service);
    let pomodoro_service_clone = Arc::clone(&pomodoro_service);
    let settings_service_clone = Arc::clone(&app_settings_service);
    let unified_mcp: StreamableHttpService<TaskMcpHandler, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(TaskMcpHandler::new(
                Arc::clone(&task_service_clone),
                Arc::clone(&pomodoro_service_clone),
                Arc::clone(&settings_service_clone),
            )),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig::default(),
        );

    let app = Router::new().nest_service(MCP_PATH, unified_mcp);

    #[cfg(feature = "plugin-runtime")]
    let app = if let Some(registry) = plugin_registry {
        use plugin::plugin_handler::PluginMcpHandler;
        const PLUGIN_MCP_PATH: &str = "/mcp/plugins";
        let plugin_mcp: StreamableHttpService<PluginMcpHandler, LocalSessionManager> =
            StreamableHttpService::new(
                move || Ok(PluginMcpHandler::new(Arc::clone(&registry))),
                LocalSessionManager::default().into(),
                StreamableHttpServerConfig::default(),
            );
        app.nest_service(PLUGIN_MCP_PATH, plugin_mcp)
    } else {
        app
    };

    app
}

#[cfg(test)]
mod tests {
    use super::{ensure_rustls_provider, mcp_url_for, start_tcp_server};
    use peekoo_task_app::NoopTaskService;
    use rmcp::{ServiceExt, transport::StreamableHttpClientTransport};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    const EXPECTED_TASK_TOOLS: &[&str] = &[
        "task_create",
        "task_list",
        "task_update",
        "task_delete",
        "task_toggle",
        "task_assign",
        "task_comment",
        "update_task_labels",
        "update_task_status",
    ];

    #[tokio::test]
    async fn http_server_exposes_all_native_tools() {
        ensure_rustls_provider();

        // Create mock services for testing
        let conn = Arc::new(std::sync::Mutex::new(
            peekoo_persistence_sqlite::setup_test_db()
        ));
        
        let (notifications, _receiver) = peekoo_notifications::NotificationService::new();
        let badges = peekoo_notifications::PeekBadgeService::new();
        let mood = peekoo_notifications::MoodReactionService::new();
        
        let pomodoro_service = Arc::new(
            peekoo_pomodoro_app::PomodoroAppService::new(
                conn.clone(),
                Arc::new(notifications),
                Arc::new(badges),
                Arc::new(mood),
            ).expect("create pomodoro service")
        );
        
        let settings_service = Arc::new(
            peekoo_app_settings::AppSettingsService::with_conn(conn).expect("create settings service")
        );

        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind listener");
        let addr = start_tcp_server(
            Arc::new(NoopTaskService), 
            pomodoro_service,
            settings_service,
            None, 
            listener
        )
            .await
            .expect("start server");

        let transport = StreamableHttpClientTransport::from_uri(mcp_url_for(addr));
        let client: rmcp::service::RunningService<rmcp::service::RoleClient, ()> =
            ().serve(transport).await.expect("complete handshake");

        let tools = client
            .list_all_tools()
            .await
            .expect("list tools without transport closing");

        let tool_names: Vec<String> = tools.into_iter().map(|t| t.name.to_string()).collect();

        // Check task tools
        for expected in EXPECTED_TASK_TOOLS {
            assert!(
                tool_names.iter().any(|n| n == expected),
                "missing task tool: {expected}"
            );
        }
        
        // Check pomodoro tools exist
        assert!(tool_names.iter().any(|n| n == "pomodoro_status"), "missing pomodoro tools");
        
        // Check settings tools exist
        assert!(tool_names.iter().any(|n| n == "settings_get_theme"), "missing settings tools");
    }
}
