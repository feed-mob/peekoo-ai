use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::flow::{OAuthFlow, OAuthFlowStatus};
use crate::url::parse_query_pairs;

const OAUTH_CALLBACK_PORT_START: u16 = 1455;
const OAUTH_CALLBACK_PORT_END: u16 = 1465;

/// Attempts to bind to an available port in the range 1455-1465.
/// Returns the bound listener and the port number, or None if all ports are in use.
fn bind_available_port() -> Option<(TcpListener, u16)> {
    for port in OAUTH_CALLBACK_PORT_START..=OAUTH_CALLBACK_PORT_END {
        match TcpListener::bind(format!("127.0.0.1:{port}")) {
            Ok(listener) => {
                log::info!("OAuth callback listener bound to port {port}");
                return Some((listener, port));
            }
            Err(err) => {
                log::debug!("Port {port} unavailable for OAuth callback: {err}");
                continue;
            }
        }
    }
    log::warn!(
        "All OAuth callback ports in range {OAUTH_CALLBACK_PORT_START}-{OAUTH_CALLBACK_PORT_END} are in use"
    );
    None
}

/// Spawns a non-blocking OAuth callback listener that accepts a single request.
/// Returns the port number if successfully bound, or None if no ports available.
pub fn spawn_callback_listener(
    flows: Arc<Mutex<HashMap<String, OAuthFlow>>>,
    flow_id: String,
) -> Option<u16> {
    let (listener, bound_port) = match bind_available_port() {
        Some(result) => result,
        None => {
            set_flow_error(
                &flows,
                &flow_id,
                format!(
                    "Failed to bind OAuth callback listener on 127.0.0.1:{OAUTH_CALLBACK_PORT_START}-{OAUTH_CALLBACK_PORT_END}: all ports in use"
                ),
            );
            return None;
        }
    };

    std::thread::spawn(move || {
        let _ = listener.set_nonblocking(true);
        let started_at = std::time::Instant::now();

        loop {
            if started_at.elapsed() > Duration::from_secs(300) {
                set_flow_error(&flows, &flow_id, "OAuth flow timed out".to_string());
                return;
            }

            match listener.accept() {
                Ok((stream, _addr)) => {
                    handle_callback_request(stream, &flows, &flow_id);
                    return;
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(err) => {
                    set_flow_error(
                        &flows,
                        &flow_id,
                        format!("OAuth callback listener error: {err}"),
                    );
                    return;
                }
            }
        }
    });

    Some(bound_port)
}

/// Handles a single OAuth callback HTTP request.
fn handle_callback_request(
    mut stream: TcpStream,
    flows: &Arc<Mutex<HashMap<String, OAuthFlow>>>,
    flow_id: &str,
) {
    let mut first_line = String::new();
    {
        let mut reader = BufReader::new(&mut stream);
        if reader.read_line(&mut first_line).is_err() {
            set_flow_error(
                flows,
                flow_id,
                "Failed to read OAuth callback request".to_string(),
            );
            return;
        }
    }

    let path = first_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();
    let query = path
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("");

    let pairs = parse_query_pairs(query);
    let code = pairs
        .iter()
        .find_map(|(k, v)| (k == "code").then(|| v.clone()));
    let state = pairs
        .iter()
        .find_map(|(k, v)| (k == "state").then(|| v.clone()));
    let oauth_error = pairs
        .iter()
        .find_map(|(k, v)| (k == "error").then(|| v.clone()));

    let mut success = false;
    let mut message = "OAuth callback received. You can close this window.".to_string();

    {
        let mut lock = match flows.lock() {
            Ok(lock) => lock,
            Err(_) => return,
        };

        let Some(flow) = lock.get_mut(flow_id) else {
            return;
        };

        if let Some(error) = oauth_error {
            flow.error = Some(format!("OAuth provider returned error: {error}"));
            flow.status = OAuthFlowStatus::Failed;
            message = "OAuth failed. You can close this window.".to_string();
        } else if code.is_none() {
            flow.error = Some("Missing OAuth authorization code".to_string());
            flow.status = OAuthFlowStatus::Failed;
            message = "OAuth failed. Missing authorization code.".to_string();
        } else if state.as_deref() != Some(flow.verifier.as_str()) {
            flow.error = Some("OAuth state mismatch".to_string());
            flow.status = OAuthFlowStatus::Failed;
            message = "OAuth failed. State mismatch.".to_string();
        } else {
            flow.auth_code = code;
            success = true;
        }
    }

    let status_line = if success {
        "HTTP/1.1 200 OK"
    } else {
        "HTTP/1.1 400 Bad Request"
    };
    let body = format!(
        "<html><body><h2>{}</h2><p>Return to Peekoo.</p></body></html>",
        message
    );
    let response = format!(
        "{status_line}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn set_flow_error(flows: &Arc<Mutex<HashMap<String, OAuthFlow>>>, flow_id: &str, error: String) {
    if let Ok(mut lock) = flows.lock()
        && let Some(flow) = lock.get_mut(flow_id)
    {
        flow.status = OAuthFlowStatus::Failed;
        flow.error = Some(error);
    }
}
