//! Common test utilities for jrow-client integration tests
//!
//! This module provides reusable mock servers and helpers for testing
//! client functionality without needing a real server.

use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{StreamExt, SinkExt};
use std::net::SocketAddr;

/// Mock WebSocket server for client testing
///
/// Provides a lightweight WebSocket server that can be used to test
/// client behavior without a full jrow-server instance.
pub struct MockWsServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
    message_rx: Option<mpsc::Receiver<String>>,
}

impl MockWsServer {
    /// Start a new mock WebSocket server
    ///
    /// The server will accept connections and echo back any messages received.
    /// Custom behavior can be implemented by providing a handler function.
    pub async fn new() -> Self {
        Self::with_handler(|msg| async move { Some(msg) }).await
    }

    /// Start a mock server with a custom message handler
    ///
    /// The handler function receives incoming messages and can return
    /// responses or None to not respond.
    pub async fn with_handler<F, Fut>(handler: F) -> Self
    where
        F: Fn(String) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Option<String>> + Send,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let (msg_tx, msg_rx) = mpsc::channel::<String>(100);

        // Spawn server task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    accept_result = listener.accept() => {
                        if let Ok((stream, _)) = accept_result {
                            let msg_tx_clone = msg_tx.clone();
                            let handler = &handler;
                            
                            tokio::spawn(async move {
                                if let Ok(ws_stream) = accept_async(stream).await {
                                    let (mut write, mut read) = ws_stream.split();
                                    
                                    while let Some(msg_result) = read.next().await {
                                        if let Ok(msg) = msg_result {
                                            if let Message::Text(text) = msg {
                                                // Send to test channel for verification
                                                let _ = msg_tx_clone.send(text.clone()).await;
                                                
                                                // Call handler and send response if any
                                                if let Some(response) = handler(text).await {
                                                    let _ = write.send(Message::Text(response)).await;
                                                }
                                            }
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            }
        });

        // Wait a bit for server to be ready
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Self {
            addr,
            shutdown_tx,
            message_rx: Some(msg_rx),
        }
    }

    /// Get the WebSocket URL for connecting to this server
    pub fn url(&self) -> String {
        format!("ws://{}", self.addr)
    }

    /// Get the bound socket address
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Wait for a message to be received by the server
    ///
    /// Returns None if the server is shut down or the timeout expires.
    pub async fn wait_for_message(&mut self) -> Option<String> {
        if let Some(rx) = &mut self.message_rx {
            tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                rx.recv()
            ).await.ok().flatten()
        } else {
            None
        }
    }

    /// Shutdown the mock server
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(()).await;
        // Give server time to clean up
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

/// Helper to create a mock JSON-RPC response
pub fn mock_response(id: i64, result: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "result": result,
        "id": id
    }).to_string()
}

/// Helper to create a mock JSON-RPC error response
pub fn mock_error_response(id: i64, code: i32, message: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": code,
            "message": message
        },
        "id": id
    }).to_string()
}

/// Helper to create a mock JSON-RPC notification
pub fn mock_notification(method: &str, params: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_creation() {
        let server = MockWsServer::new().await;
        assert!(!server.url().is_empty());
        assert!(server.url().starts_with("ws://127.0.0.1:"));
        server.shutdown().await;
    }

    #[test]
    fn test_mock_response_format() {
        let response = mock_response(1, serde_json::json!({"value": 42}));
        assert!(response.contains("\"jsonrpc\":\"2.0\""));
        assert!(response.contains("\"id\":1"));
        assert!(response.contains("\"result\""));
    }

    #[test]
    fn test_mock_error_response_format() {
        let response = mock_error_response(1, -32601, "Method not found");
        assert!(response.contains("\"error\""));
        assert!(response.contains("-32601"));
        assert!(response.contains("Method not found"));
    }

    #[test]
    fn test_mock_notification_format() {
        let notification = mock_notification("event", serde_json::json!({"data": "test"}));
        assert!(notification.contains("\"method\":\"event\""));
        assert!(notification.contains("\"params\""));
        assert!(!notification.contains("\"id\""));
    }
}

