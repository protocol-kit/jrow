//! Request tracking for JSON-RPC client
//!
//! This module manages the lifecycle of outgoing JSON-RPC requests,
//! correlating requests with their eventual responses.
//!
//! # Request Lifecycle
//!
//! 1. **Generate ID**: Assign unique ID to request
//! 2. **Register**: Create oneshot channel for response
//! 3. **Send**: Transmit request over WebSocket
//! 4. **Wait**: Caller awaits on the oneshot receiver
//! 5. **Receive**: Server response arrives via WebSocket
//! 6. **Complete**: Match response ID, send via channel
//! 7. **Return**: Caller receives response, returns to user
//!
//! # Why Oneshot Channels?
//!
//! Each request gets a dedicated oneshot channel because:
//! - Responses arrive asynchronously and out-of-order
//! - Channels provide natural async/await integration
//! - Oneshot cleanup is automatic (no memory leaks)
//!
//! # Timeouts
//!
//! Request timeouts are implemented at a higher level by racing
//! the receiver against a `tokio::time::timeout`.

use jrow_core::{Error, Id, JsonRpcResponse, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

/// Pending request waiting for a response
pub struct PendingRequest {
    /// Channel to send the response
    pub tx: oneshot::Sender<Result<JsonRpcResponse>>,
}

/// Manager for tracking pending requests
#[derive(Clone)]
pub struct RequestManager {
    /// Map of request ID to pending request
    pending: Arc<Mutex<HashMap<String, PendingRequest>>>,
    /// Counter for generating request IDs
    counter: Arc<Mutex<u64>>,
}

impl RequestManager {
    /// Create a new request manager
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
            counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Generate a new unique request ID
    pub async fn next_id(&self) -> Id {
        let mut counter = self.counter.lock().await;
        let id = *counter;
        *counter += 1;
        Id::Number(id as i64)
    }

    /// Register a pending request
    pub async fn register(&self, id: Id) -> oneshot::Receiver<Result<JsonRpcResponse>> {
        let (tx, rx) = oneshot::channel();
        let pending_req = PendingRequest { tx };

        let id_str = id_to_string(&id);
        self.pending.lock().await.insert(id_str, pending_req);

        rx
    }

    /// Complete a pending request with a response
    pub async fn complete(&self, id: &Id, response: JsonRpcResponse) {
        let id_str = id_to_string(id);
        if let Some(pending) = self.pending.lock().await.remove(&id_str) {
            let _ = pending.tx.send(Ok(response));
        }
    }

    /// Fail a pending request with an error
    #[allow(dead_code)]
    pub async fn fail(&self, id: &Id, error: Error) {
        let id_str = id_to_string(id);
        if let Some(pending) = self.pending.lock().await.remove(&id_str) {
            let _ = pending.tx.send(Err(error));
        }
    }

    /// Fail all pending requests
    pub async fn fail_all(&self, error: Error) {
        let mut pending = self.pending.lock().await;
        for (_, req) in pending.drain() {
            let _ = req.tx.send(Err(error.clone()));
        }
    }

    /// Get the number of pending requests
    #[allow(dead_code)]
    pub async fn pending_count(&self) -> usize {
        self.pending.lock().await.len()
    }
}

impl Default for RequestManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert an ID to a string key for the hashmap
fn id_to_string(id: &Id) -> String {
    match id {
        Id::String(s) => s.clone(),
        Id::Number(n) => n.to_string(),
        Id::Null => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_request_manager() {
        let manager = RequestManager::new();

        // Generate IDs
        let id1 = manager.next_id().await;
        let id2 = manager.next_id().await;

        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_register_and_complete() {
        let manager = RequestManager::new();
        let id = Id::Number(1);

        let rx = manager.register(id.clone()).await;
        assert_eq!(manager.pending_count().await, 1);

        let response = JsonRpcResponse::success(serde_json::json!(42), id.clone());
        manager.complete(&id, response.clone()).await;

        assert_eq!(manager.pending_count().await, 0);

        let result = rx.await.unwrap().unwrap();
        assert_eq!(result.result, Some(serde_json::json!(42)));
    }

    #[tokio::test]
    async fn test_fail_request() {
        let manager = RequestManager::new();
        let id = Id::Number(1);

        let rx = manager.register(id.clone()).await;
        manager.fail(&id, Error::Timeout).await;

        let result = rx.await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fail_all() {
        let manager = RequestManager::new();

        let rx1 = manager.register(Id::Number(1)).await;
        let rx2 = manager.register(Id::Number(2)).await;

        assert_eq!(manager.pending_count().await, 2);

        manager.fail_all(Error::ConnectionClosed).await;

        assert_eq!(manager.pending_count().await, 0);
        assert!(rx1.await.unwrap().is_err());
        assert!(rx2.await.unwrap().is_err());
    }
}
