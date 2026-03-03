use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::flow::{OAuthFlow, OAuthFlowStatus};
use crate::url::parse_query_pairs;

pub fn spawn_callback_listener(flows: Arc<Mutex<HashMap<String, OAuthFlow>>>, flow_id: String) {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind("127.0.0.1:1455") {
            Ok(listener) => listener,
            Err(err) => {
                set_flow_error(
                    &flows,
                    &flow_id,
                    format!("Failed to bind OAuth callback listener on 127.0.0.1:1455: {err}"),
                );
                return;
            }
        };

        let _ = listener.set_nonblocking(true);
        let started_at = std::time::Instant::now();

        loop {
            if started_at.elapsed() > Duration::from_secs(300) {
                set_flow_error(&flows, &flow_id, "OAuth flow timed out".to_string());
                return;
            }

            match listener.accept() {
                Ok((mut stream, _addr)) => {
                    let mut first_line = String::new();
                    {
                        let mut reader = BufReader::new(&mut stream);
                        if reader.read_line(&mut first_line).is_err() {
                            set_flow_error(
                                &flows,
                                &flow_id,
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
                    let mut message =
                        "OAuth callback received. You can close this window.".to_string();

                    {
                        let mut lock = match flows.lock() {
                            Ok(lock) => lock,
                            Err(_) => return,
                        };

                        let Some(flow) = lock.get_mut(&flow_id) else {
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
}

fn set_flow_error(flows: &Arc<Mutex<HashMap<String, OAuthFlow>>>, flow_id: &str, error: String) {
    if let Ok(mut lock) = flows.lock()
        && let Some(flow) = lock.get_mut(flow_id)
    {
        flow.status = OAuthFlowStatus::Failed;
        flow.error = Some(error);
    }
}
