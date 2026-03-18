//! WebSocket helpers for plugins.
//!
//! Requires the `net:websocket` permission.

use extism_pdk::{Error, Json};

use crate::host_fns::{
    peekoo_websocket_close, peekoo_websocket_connect, peekoo_websocket_recv, peekoo_websocket_send,
    WebSocketCloseRequest, WebSocketConnectRequest, WebSocketRecvRequest, WebSocketSendRequest,
};

pub fn connect(url: &str) -> Result<String, Error> {
    let response = unsafe {
        peekoo_websocket_connect(Json(WebSocketConnectRequest {
            url: url.to_string(),
        }))?
    };
    Ok(response.0.socket_id)
}

pub fn send(socket_id: &str, text: &str) -> Result<(), Error> {
    unsafe {
        peekoo_websocket_send(Json(WebSocketSendRequest {
            socket_id: socket_id.to_string(),
            text: text.to_string(),
        }))?
    };
    Ok(())
}

pub fn recv(socket_id: &str) -> Result<String, Error> {
    let response = unsafe {
        peekoo_websocket_recv(Json(WebSocketRecvRequest {
            socket_id: socket_id.to_string(),
        }))?
    };
    Ok(response.0.text)
}

pub fn close(socket_id: &str) -> Result<(), Error> {
    unsafe {
        peekoo_websocket_close(Json(WebSocketCloseRequest {
            socket_id: socket_id.to_string(),
        }))?
    };
    Ok(())
}
