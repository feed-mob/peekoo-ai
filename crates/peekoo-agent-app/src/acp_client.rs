use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use agent_client_protocol::{
    Agent as _, Client, ClientSideConnection, ContentBlock, InitializeRequest, McpServer,
    McpServerHttp, NewSessionRequest, PromptRequest, ProtocolVersion, SessionNotification,
    SessionUpdate, StopReason,
};
use tokio::process::Command;
use tokio::task::LocalSet;
use tokio::time::{Duration, timeout};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub(crate) struct AcpPromptResult {
    pub text: String,
    pub stop_reason: StopReason,
}

#[derive(Clone)]
struct PromptClient {
    context: String,
    output: Arc<Mutex<String>>,
}

#[async_trait::async_trait(?Send)]
impl Client for PromptClient {
    async fn request_permission(
        &self,
        args: agent_client_protocol::RequestPermissionRequest,
    ) -> Result<agent_client_protocol::RequestPermissionResponse, agent_client_protocol::Error>
    {
        if let Some(option) = args.options.iter().find(|option| {
            matches!(
                option.kind,
                agent_client_protocol::PermissionOptionKind::AllowOnce
                    | agent_client_protocol::PermissionOptionKind::AllowAlways
            )
        }) {
            Ok(agent_client_protocol::RequestPermissionResponse::new(
                agent_client_protocol::RequestPermissionOutcome::Selected(
                    agent_client_protocol::SelectedPermissionOutcome::new(option.option_id.clone()),
                ),
            ))
        } else {
            Ok(agent_client_protocol::RequestPermissionResponse::new(
                agent_client_protocol::RequestPermissionOutcome::Cancelled,
            ))
        }
    }

    async fn session_notification(
        &self,
        args: SessionNotification,
    ) -> Result<(), agent_client_protocol::Error> {
        match args.update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let ContentBlock::Text(text) = chunk.content {
                    if let Ok(mut guard) = self.output.lock() {
                        guard.push_str(&text.text);
                    }
                    tracing::debug!(
                        "ACP [{}] chunk: {}",
                        self.context,
                        text.text.chars().take(200).collect::<String>()
                    );
                }
            }
            SessionUpdate::ToolCall(tool_call) => {
                tracing::info!(
                    "ACP [{}] tool call: {} ({:?})",
                    self.context,
                    tool_call.title,
                    tool_call.kind
                );
            }
            SessionUpdate::ToolCallUpdate(update) => {
                tracing::info!("ACP [{}] tool update: {:?}", self.context, update.fields);
            }
            _ => {}
        }
        Ok(())
    }
}

pub(crate) fn resolve_acp_command_path(
    bundled_acp_path: Option<PathBuf>,
    current_exe: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = bundled_acp_path.filter(|path| path.exists() && path.is_file()) {
        return path;
    }

    let bin_name = if cfg!(windows) {
        "peekoo-agent-acp.exe"
    } else {
        "peekoo-agent-acp"
    };

    current_exe
        .and_then(|exe| exe.parent().map(|p| p.join(bin_name)))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from(bin_name))
}

pub(crate) fn build_session_mcp_servers(
    mcp_address: Option<std::net::SocketAddr>,
) -> Vec<McpServer> {
    mcp_address
        .map(|addr| {
            let base_url = peekoo_mcp_server::mcp_url_for(addr);
            let plugins_url = format!("{}/plugins", base_url);
            vec![
                McpServer::Http(McpServerHttp::new("peekoo-native-tools", base_url)),
                McpServer::Http(McpServerHttp::new("peekoo-plugin-tools", plugins_url)),
            ]
        })
        .unwrap_or_default()
}

pub(crate) async fn run_prompt_and_collect(
    context: &str,
    content_blocks: Vec<ContentBlock>,
    launch_env: Vec<(String, String)>,
    mcp_address: Option<std::net::SocketAddr>,
    bundled_acp_path: Option<PathBuf>,
    prompt_timeout: Option<Duration>,
) -> Result<AcpPromptResult, String> {
    let command_path = resolve_acp_command_path(bundled_acp_path, std::env::current_exe().ok());
    let output = Arc::new(Mutex::new(String::new()));

    let mut cmd = Command::new(&command_path);
    if let Some(addr) = mcp_address {
        cmd.env("PEEKOO_MCP_PORT", addr.port().to_string())
            .env("PEEKOO_MCP_HOST", addr.ip().to_string());
    }
    for (key, value) in launch_env {
        cmd.env(key, value);
    }

    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Spawn ACP process error: {e}"))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "ACP stdin unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "ACP stdout unavailable".to_string())?;

    let local_set = LocalSet::new();
    let output_clone = Arc::clone(&output);
    let context_label = context.to_string();

    let prompt_result = local_set
        .run_until(async move {
            let (conn, handle_io) = ClientSideConnection::new(
                PromptClient {
                    context: context_label.clone(),
                    output: output_clone,
                },
                stdin.compat_write(),
                stdout.compat(),
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );

            tokio::task::spawn_local(async move {
                if let Err(err) = handle_io.await {
                    tracing::warn!("ACP [{}] I/O error: {}", context_label, err);
                }
            });

            conn.initialize(InitializeRequest::new(ProtocolVersion::V1))
                .await
                .map_err(|e| format!("ACP initialize error: {e}"))?;

            let session = conn
                .new_session(
                    NewSessionRequest::new(std::env::current_dir().unwrap_or_default())
                        .mcp_servers(build_session_mcp_servers(mcp_address)),
                )
                .await
                .map_err(|e| format!("ACP new_session error: {e}"))?;

            let response = if let Some(limit) = prompt_timeout {
                timeout(
                    limit,
                    conn.prompt(PromptRequest::new(session.session_id, content_blocks)),
                )
                .await
                .map_err(|_| format!("ACP prompt timed out after {}s", limit.as_secs()))?
                .map_err(|e| format!("ACP prompt error: {e}"))?
            } else {
                conn.prompt(PromptRequest::new(session.session_id, content_blocks))
                    .await
                    .map_err(|e| format!("ACP prompt error: {e}"))?
            };

            Ok::<_, String>(response.stop_reason)
        })
        .await;

    let _ = child.kill().await;

    let stop_reason = prompt_result?;
    let text = output
        .lock()
        .map_err(|e| format!("ACP output lock error: {e}"))?
        .clone();

    Ok(AcpPromptResult { text, stop_reason })
}

pub(crate) fn run_prompt_and_collect_blocking(
    context: &str,
    content_blocks: Vec<ContentBlock>,
    launch_env: Vec<(String, String)>,
    mcp_address: Option<std::net::SocketAddr>,
    bundled_acp_path: Option<PathBuf>,
    prompt_timeout: Option<Duration>,
) -> Result<AcpPromptResult, String> {
    let context = context.to_string();

    std::thread::spawn(move || -> Result<AcpPromptResult, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Create tokio runtime error: {e}"))?;

        runtime.block_on(run_prompt_and_collect(
            &context,
            content_blocks,
            launch_env,
            mcp_address,
            bundled_acp_path,
            prompt_timeout,
        ))
    })
    .join()
    .map_err(|_| "ACP client thread panicked".to_string())?
}

#[cfg(test)]
mod tests {
    use super::{build_session_mcp_servers, resolve_acp_command_path};

    #[test]
    fn builds_http_mcp_server_for_session() {
        let servers = build_session_mcp_servers(Some(([127, 0, 0, 1], 49152).into()));
        let serialized = serde_json::to_value(&servers).expect("serialize mcp servers");
        assert_eq!(serialized[0]["type"], "http");
        assert_eq!(serialized[0]["name"], "peekoo-native-tools");
        assert_eq!(serialized[0]["url"], "http://127.0.0.1:49152/mcp");
    }

    #[test]
    fn resolve_acp_command_path_prefers_explicit_bundled_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundled = temp.path().join("peekoo-agent-acp");
        std::fs::write(&bundled, "").expect("create bundled binary");

        let resolved =
            resolve_acp_command_path(Some(bundled.clone()), Some(temp.path().join("desktop")));

        assert_eq!(resolved, bundled);
    }

    #[test]
    fn resolve_acp_command_path_uses_sibling_binary_when_present() {
        let temp = tempfile::tempdir().expect("tempdir");
        let exe_dir = temp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).expect("create bin dir");
        let current_exe = exe_dir.join("peekoo-desktop");
        let sibling = exe_dir.join(if cfg!(windows) {
            "peekoo-agent-acp.exe"
        } else {
            "peekoo-agent-acp"
        });
        std::fs::write(&sibling, "").expect("create sibling binary");

        let resolved = resolve_acp_command_path(None, Some(current_exe));

        assert_eq!(resolved, sibling);
    }
}
