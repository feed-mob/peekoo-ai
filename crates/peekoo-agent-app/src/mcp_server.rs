//! Shared MCP Server Manager
//!
//! Manages a single MCP server instance that runs for the lifetime of the application.
//! All agent ACP processes connect to this shared server.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use peekoo_app_settings::AppSettingsService;
use peekoo_mcp_server::{mcp_url_for, start_tcp_server};
use peekoo_plugin_host::PluginRegistry;
use peekoo_pomodoro_app::PomodoroAppService;
use peekoo_task_app::TaskService;

/// Port range for MCP server
const MCP_PORT_RANGE_START: u16 = 49152;
const MCP_PORT_RANGE_END: u16 = 65535;

/// Global MCP server state.
/// Stores (address, shutdown_token) once initialized.
/// The MCP server runs on its own dedicated thread with its own tokio runtime.
pub static MCP_SERVER_STATE: LazyLock<std::sync::Mutex<Option<(SocketAddr, CancellationToken)>>> =
    LazyLock::new(|| std::sync::Mutex::new(None));

/// Get the MCP server address if already started.
pub fn get_mcp_address() -> Option<SocketAddr> {
    MCP_SERVER_STATE
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|(addr, _)| *addr))
}

/// Get the full MCP server URL if already started (e.g. `http://127.0.0.1:49152/mcp`).
pub fn get_mcp_url() -> Option<String> {
    get_mcp_address().map(mcp_url_for)
}

/// Get the MCP plugins endpoint URL if already started (e.g. `http://127.0.0.1:49152/mcp/plugins`).
pub fn get_mcp_plugins_url() -> Option<String> {
    get_mcp_address().map(|addr| format!("{}/plugins", mcp_url_for(addr)))
}

/// Start the MCP server synchronously on a dedicated thread.
///
/// This can be called from outside a tokio runtime (e.g., during app startup).
/// Returns the bound address on success.
pub fn start_sync(
    task_service: Arc<dyn TaskService>,
    pomodoro_service: Arc<PomodoroAppService>,
    app_settings_service: Arc<AppSettingsService>,
    plugin_registry: Option<Arc<PluginRegistry>>,
    shutdown_token: CancellationToken,
) -> Result<SocketAddr, String> {
    // Check if already started
    if let Some(addr) = get_mcp_address() {
        eprintln!("[peekoo][mcp] already running at {}", mcp_url_for(addr));
        tracing::debug!("🔗 [MCP] Server already running at {}", mcp_url_for(addr));
        return Ok(addr);
    }

    eprintln!("[peekoo][mcp] spawning dedicated MCP thread");
    tracing::info!("🚀 [MCP] Starting server on dedicated thread...");

    // Channel to receive the bound address from the spawned thread
    let (addr_tx, addr_rx) = std::sync::mpsc::channel::<Result<SocketAddr, String>>();
    let token_for_thread = shutdown_token.clone();

    // Spawn MCP server on its own thread with dedicated runtime
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = addr_tx.send(Err(format!("Failed to create MCP runtime: {}", e)));
                return;
            }
        };

        rt.block_on(async {
            match McpServerManager::start(
                task_service,
                pomodoro_service,
                app_settings_service,
                plugin_registry,
                token_for_thread.clone(),
            )
                .await
            {
                Ok(manager) => {
                    let addr = manager.address();
                    tracing::info!("✅ [MCP] Server running at {}", mcp_url_for(addr));
                    let _ = addr_tx.send(Ok(addr));
                    // Keep the runtime alive until shutdown
                    token_for_thread.cancelled().await;
                    tracing::info!("🛑 [MCP] Server shutdown complete");
                }
                Err(e) => {
                    let _ = addr_tx.send(Err(format!("Failed to start MCP server: {}", e)));
                }
            }
        });
    });

    // Wait for the server to bind and report its address
    match addr_rx.recv() {
        Ok(Ok(addr)) => {
            eprintln!("[peekoo][mcp] confirmed ready at {}", mcp_url_for(addr));
            tracing::info!("✅ [MCP] Server confirmed ready at {}", mcp_url_for(addr));

            // Store in global state
            if let Ok(mut guard) = MCP_SERVER_STATE.lock() {
                *guard = Some((addr, shutdown_token));
            }

            Ok(addr)
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err("MCP server thread panicked".to_string()),
    }
}

/// Shutdown the MCP server if running.
pub fn shutdown() {
    if let Ok(guard) = MCP_SERVER_STATE.lock()
        && let Some((addr, ref token)) = *guard
    {
        tracing::info!("🛑 [MCP] Shutting down server at {}", mcp_url_for(addr));
        token.cancel();
    }
}

/// Shared MCP server manager that runs for the lifetime of the application.
pub struct McpServerManager {
    address: SocketAddr,
}

impl McpServerManager {
    /// Start the MCP server and return the manager with the server address.
    ///
    /// The server runs until the cancellation token is triggered.
    async fn start(
        task_service: Arc<dyn TaskService>,
        pomodoro_service: Arc<PomodoroAppService>,
        app_settings_service: Arc<AppSettingsService>,
        plugin_registry: Option<Arc<PluginRegistry>>,
        shutdown_token: CancellationToken,
    ) -> Result<Self, String> {
        // Find an available port and bind immediately (avoids race condition)
        let listener = Self::find_available_listener().await?;

        let actual_address = listener
            .local_addr()
            .map_err(|e| format!("Failed to get local address: {}", e))?;

        tracing::info!("🚀 [MCP] Binding server on {}", mcp_url_for(actual_address));

        // Start the MCP server (takes ownership of listener)
        let server_address = start_tcp_server(
            task_service,
            pomodoro_service,
            app_settings_service,
            plugin_registry,
            listener,
        )
            .await
            .map_err(|e| format!("Failed to start MCP server: {}", e))?;

        tracing::info!(
            "✅ [MCP] Server listening at {}",
            mcp_url_for(server_address)
        );
        tracing::info!(
            "📋 [MCP] Available tools: 24 native tools (task, pomodoro, settings) \
             (+ plugin tools if registry provided)"
        );

        // Watch for shutdown signal
        let address = server_address;
        tokio::spawn(async move {
            shutdown_token.cancelled().await;
            tracing::info!("🛑 [MCP] Server shutting down ({})", mcp_url_for(address));
        });

        Ok(Self {
            address: server_address,
        })
    }

    /// Get the MCP server address (host:port)
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Find an available TCP port and return a bound listener.
    /// This avoids the race condition of bind-check-release-rebind.
    async fn find_available_listener() -> Result<TcpListener, String> {
        for port in MCP_PORT_RANGE_START..=MCP_PORT_RANGE_END {
            let addr = SocketAddr::from(([127, 0, 0, 1], port));
            match TcpListener::bind(addr).await {
                Ok(listener) => {
                    tracing::debug!("🚀 [MCP] Bound to port {}", port);
                    return Ok(listener);
                }
                Err(e) => {
                    tracing::debug!("🚀 [MCP] Port {} unavailable: {}", port, e);
                    continue;
                }
            }
        }
        Err("No available port found for MCP server in range 49152-65535".to_string())
    }
}
