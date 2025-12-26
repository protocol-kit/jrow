//! Batch request building and response handling
//!
//! JSON-RPC 2.0 allows sending multiple requests in a single message array.
//! This module provides builders for constructing batch requests and handling
//! their responses efficiently.
//!
//! # Benefits of Batching
//!
//! - **Reduced overhead**: Single WebSocket message for multiple operations
//! - **Better throughput**: Network RTT amortized across many requests
//! - **Atomic groups**: Related operations sent together
//!
//! # Usage Pattern
//!
//! 1. Create a `BatchRequest` builder
//! 2. Add requests and/or notifications
//! 3. Send the batch to the server
//! 4. Parse responses using `BatchResponse`
//!
//! # Examples
//!
//! ```rust,no_run
//! use jrow_client::{JrowClient, BatchRequest};
//!
//! # async fn example(client: &JrowClient) -> jrow_core::Result<()> {
//! let mut batch = BatchRequest::new();
//!
//! // Add requests (expect responses)
//! let id1 = batch.add_request("method1", serde_json::json!({"a": 1}));
//! let id2 = batch.add_request("method2", serde_json::json!({"b": 2}));
//!
//! // Add notification (no response)
//! batch.add_notification("notify", serde_json::json!({"event": "test"}));
//!
//! // Send batch
//! let responses = client.batch(batch).await?;
//!
//! // Extract results by ID
//! let result1: serde_json::Value = responses.get(&id1)?;
//! let result2: serde_json::Value = responses.get(&id2)?;
//! # Ok(())
//! # }
//! ```

use jrow_core::{Error, Id, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Builder for constructing batch requests
#[derive(Debug)]
pub struct BatchRequest {
    requests: Vec<JsonRpcRequest>,
    notifications: Vec<JsonRpcNotification>,
    counter: AtomicU64,
}

impl BatchRequest {
    /// Create a new batch request builder
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
            notifications: Vec::new(),
            counter: AtomicU64::new(1),
        }
    }

    /// Add a request to the batch and return its ID
    pub fn add_request<P>(&mut self, method: impl Into<String>, params: P) -> Id
    where
        P: Serialize,
    {
        let id_num = self.counter.fetch_add(1, Ordering::SeqCst);
        let id = Id::Number(id_num as i64);

        let params_value = serde_json::to_value(params).ok();
        let request = JsonRpcRequest::new(method, params_value, id.clone());

        self.requests.push(request);
        id
    }

    /// Add a notification to the batch (no response expected)
    pub fn add_notification<P>(&mut self, method: impl Into<String>, params: P)
    where
        P: Serialize,
    {
        let params_value = serde_json::to_value(params).ok();
        let notification = JsonRpcNotification::new(method, params_value);

        self.notifications.push(notification);
    }

    /// Get the total number of items in the batch
    pub fn len(&self) -> usize {
        self.requests.len() + self.notifications.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all requests (for internal use)
    pub(crate) fn requests(&self) -> &[JsonRpcRequest] {
        &self.requests
    }

    /// Get all notifications (for internal use)
    pub(crate) fn notifications(&self) -> &[JsonRpcNotification] {
        &self.notifications
    }

    /// Get request IDs (for tracking)
    pub fn request_ids(&self) -> Vec<Id> {
        self.requests.iter().map(|r| r.id.clone()).collect()
    }
}

impl Default for BatchRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response handler for batch requests
#[derive(Debug)]
pub struct BatchResponse {
    responses: HashMap<String, JsonRpcResponse>,
}

impl BatchResponse {
    /// Create a new batch response from a list of responses
    pub fn new(responses: Vec<JsonRpcResponse>) -> Self {
        let mut map = HashMap::new();
        for response in responses {
            let key = id_to_key(&response.id);
            map.insert(key, response);
        }
        Self { responses: map }
    }

    /// Get a typed result for a specific request ID
    pub fn get<R>(&self, id: &Id) -> Result<R>
    where
        R: for<'de> Deserialize<'de>,
    {
        let key = id_to_key(id);
        let response = self
            .responses
            .get(&key)
            .ok_or_else(|| Error::Internal(format!("No response for ID: {}", id)))?;

        if let Some(error) = &response.error {
            return Err(Error::JsonRpc(error.clone()));
        }

        let result = response
            .result
            .as_ref()
            .ok_or_else(|| Error::Internal("Response missing result".to_string()))?;

        serde_json::from_value(result.clone()).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Get the raw response for a specific request ID
    pub fn get_response(&self, id: &Id) -> Option<&JsonRpcResponse> {
        let key = id_to_key(id);
        self.responses.get(&key)
    }

    /// Check if a response exists for an ID
    pub fn has_response(&self, id: &Id) -> bool {
        let key = id_to_key(id);
        self.responses.contains_key(&key)
    }

    /// Get all response IDs
    pub fn response_ids(&self) -> Vec<Id> {
        self.responses.values().map(|r| r.id.clone()).collect()
    }

    /// Get the number of responses
    pub fn len(&self) -> usize {
        self.responses.len()
    }

    /// Check if there are no responses
    pub fn is_empty(&self) -> bool {
        self.responses.is_empty()
    }

    /// Check if all requests succeeded
    pub fn all_success(&self) -> bool {
        self.responses.values().all(|r| r.error.is_none())
    }

    /// Get all errors in the batch
    pub fn errors(&self) -> Vec<(&Id, &jrow_core::JsonRpcErrorData)> {
        self.responses
            .values()
            .filter_map(|r| r.error.as_ref().map(|e| (&r.id, e)))
            .collect()
    }
}

/// Convert an ID to a string key for HashMap
fn id_to_key(id: &Id) -> String {
    match id {
        Id::String(s) => format!("s:{}", s),
        Id::Number(n) => format!("n:{}", n),
        Id::Null => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_request_builder() {
        let mut batch = BatchRequest::new();

        let id1 = batch.add_request("method1", serde_json::json!({"a": 1}));
        let id2 = batch.add_request("method2", serde_json::json!({"b": 2}));
        batch.add_notification("notify", serde_json::json!({"c": 3}));

        assert_eq!(batch.len(), 3);
        assert_eq!(batch.requests().len(), 2);
        assert_eq!(batch.notifications().len(), 1);

        let ids = batch.request_ids();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn test_batch_response() {
        let response1 = JsonRpcResponse::success(serde_json::json!(42), Id::Number(1));
        let response2 = JsonRpcResponse::success(serde_json::json!(100), Id::Number(2));

        let batch_resp = BatchResponse::new(vec![response1, response2]);

        assert_eq!(batch_resp.len(), 2);
        assert!(batch_resp.has_response(&Id::Number(1)));
        assert!(batch_resp.has_response(&Id::Number(2)));

        let val1: i32 = batch_resp.get(&Id::Number(1)).unwrap();
        let val2: i32 = batch_resp.get(&Id::Number(2)).unwrap();

        assert_eq!(val1, 42);
        assert_eq!(val2, 100);
    }

    #[test]
    fn test_batch_response_with_error() {
        let response1 = JsonRpcResponse::success(serde_json::json!(42), Id::Number(1));
        let response2 = JsonRpcResponse::error(
            jrow_core::JsonRpcErrorData::method_not_found("test"),
            Id::Number(2),
        );

        let batch_resp = BatchResponse::new(vec![response1, response2]);

        assert_eq!(batch_resp.len(), 2);
        assert!(!batch_resp.all_success());

        let val1: Result<i32> = batch_resp.get(&Id::Number(1));
        assert!(val1.is_ok());

        let val2: Result<i32> = batch_resp.get(&Id::Number(2));
        assert!(val2.is_err());

        let errors = batch_resp.errors();
        assert_eq!(errors.len(), 1);
    }
}


