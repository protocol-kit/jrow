//! Batch processing for JSON-RPC requests
//!
//! JSON-RPC 2.0 allows multiple requests to be sent in a single message as an array.
//! This module provides efficient batch processing with configurable execution modes.
//!
//! # Batch Modes
//!
//! - **Parallel**: Execute all requests concurrently for maximum throughput
//! - **Sequential**: Execute requests in order, useful when requests have dependencies
//!
//! # Size Limiting
//!
//! To prevent denial-of-service via extremely large batches, you can configure
//! a maximum batch size. Batches exceeding the limit return an error response.
//!
//! # Examples
//!
//! ```rust
//! use jrow_server::{BatchMode, BatchProcessor};
//!
//! // Parallel processing with 100-request limit
//! let processor = BatchProcessor::with_limit(BatchMode::Parallel, Some(100));
//!
//! // Sequential processing, unlimited size
//! let sequential = BatchProcessor::new(BatchMode::Sequential);
//! ```

use crate::{Router, SubscriptionManager};
use jrow_core::{codec, JsonRpcErrorData, JsonRpcMessage, JsonRpcResponse};

/// Mode for processing batch requests
///
/// Determines whether batch requests are executed concurrently or in order.
///
/// # Trade-offs
///
/// - **Parallel**: Faster overall, but requests may complete out of order
/// - **Sequential**: Preserves order, necessary if later requests depend on earlier ones
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchMode {
    /// Process all requests concurrently (unordered)
    ///
    /// This is the default and recommended mode for maximum throughput.
    /// Responses are collected and returned in the same order as requests,
    /// but execution happens concurrently.
    Parallel,
    
    /// Process requests sequentially in order
    ///
    /// Use this when requests have dependencies (e.g., second request
    /// uses data created by first request).
    Sequential,
}

impl Default for BatchMode {
    fn default() -> Self {
        BatchMode::Parallel
    }
}

/// Processor for handling batch requests
#[derive(Clone)]
pub struct BatchProcessor {
    mode: BatchMode,
    max_size: Option<usize>,
}

impl BatchProcessor {
    /// Create a new batch processor with the specified mode
    pub fn new(mode: BatchMode) -> Self {
        Self { mode, max_size: None }
    }

    /// Create a new batch processor with mode and max batch size
    pub fn with_limit(mode: BatchMode, max_size: Option<usize>) -> Self {
        Self { mode, max_size }
    }

    /// Process a batch of JSON-RPC messages
    #[tracing::instrument(skip(self, batch_values, router, sub_manager), fields(batch_size = batch_values.len(), mode = ?self.mode, conn_id = conn_id))]
    pub async fn process_batch(
        &self,
        batch_values: Vec<serde_json::Value>,
        router: &Router,
        conn_id: u64,
        sub_manager: &SubscriptionManager,
    ) -> Vec<JsonRpcResponse> {
        // Check batch size limit
        if let Some(max_size) = self.max_size {
            if batch_values.len() > max_size {
                tracing::warn!(
                    batch_size = batch_values.len(),
                    max_size = max_size,
                    "Batch size exceeded"
                );
                // Return a single error response for batch size exceeded
                return vec![JsonRpcResponse::error(
                    JsonRpcErrorData::batch_size_exceeded(max_size, batch_values.len()),
                    jrow_core::Id::Null,
                )];
            }
        }

        // Decode all messages
        let messages = codec::decode_batch_messages(batch_values);

        let responses = match self.mode {
            BatchMode::Parallel => {
                self.process_parallel(messages, router, conn_id, sub_manager)
                    .await
            }
            BatchMode::Sequential => {
                self.process_sequential(messages, router, conn_id, sub_manager)
                    .await
            }
        };
        
        tracing::debug!(response_count = responses.len(), "Batch processing completed");
        responses
    }

    /// Process batch requests in parallel
    async fn process_parallel(
        &self,
        messages: Vec<Result<JsonRpcMessage, jrow_core::Error>>,
        router: &Router,
        conn_id: u64,
        sub_manager: &SubscriptionManager,
    ) -> Vec<JsonRpcResponse> {
        let mut tasks = Vec::new();

        for msg_result in messages {
            let router = router.clone();
            let sub_manager = sub_manager.clone();

            tasks.push(tokio::spawn(async move {
                process_single_message(msg_result, &router, conn_id, &sub_manager).await
            }));
        }

        // Wait for all tasks and collect results
        let mut responses = Vec::new();
        for task in tasks {
            if let Ok(Some(response)) = task.await {
                responses.push(response);
            }
        }

        responses
    }

    /// Process batch requests sequentially
    async fn process_sequential(
        &self,
        messages: Vec<Result<JsonRpcMessage, jrow_core::Error>>,
        router: &Router,
        conn_id: u64,
        sub_manager: &SubscriptionManager,
    ) -> Vec<JsonRpcResponse> {
        let mut responses = Vec::new();

        for msg_result in messages {
            if let Some(response) =
                process_single_message(msg_result, router, conn_id, sub_manager).await
            {
                responses.push(response);
            }
        }

        responses
    }
}

/// Process a single message from a batch
async fn process_single_message(
    msg_result: Result<JsonRpcMessage, jrow_core::Error>,
    router: &Router,
    conn_id: u64,
    sub_manager: &SubscriptionManager,
) -> Option<JsonRpcResponse> {
    match msg_result {
        Ok(JsonRpcMessage::Request(request)) => {
            // Process request - use the same logic as connection.rs
            Some(
                crate::connection::process_request_for_batch(request, router, conn_id, sub_manager)
                    .await,
            )
        }
        Ok(JsonRpcMessage::Notification(notification)) => {
            // Process notification but don't return a response
            if let Err(e) = router
                .route_with_conn_id(&notification.method, notification.params, conn_id)
                .await
            {
                eprintln!("Error processing notification in batch: {}", e);
            }
            None
        }
        Ok(JsonRpcMessage::Batch(_)) => {
            // Nested batches are not allowed per spec
            Some(JsonRpcResponse::error(
                JsonRpcErrorData::invalid_request("Nested batches are not allowed"),
                jrow_core::Id::Null,
            ))
        }
        Ok(JsonRpcMessage::Response(_)) => {
            // Responses in batch requests don't make sense
            None
        }
        Err(e) => {
            // Parse error for this message
            Some(JsonRpcResponse::error(
                JsonRpcErrorData::invalid_request(e.to_string()),
                jrow_core::Id::Null,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::from_fn;
    use jrow_core::{Id, JsonRpcRequest};

    #[tokio::test]
    async fn test_parallel_batch() {
        let mut router = Router::new();
        let handler = from_fn(|_| async { Ok(serde_json::json!({"result": "ok"})) });
        router.register("test", handler);

        let sub_manager = SubscriptionManager::new();
        let processor = BatchProcessor::new(BatchMode::Parallel);

        let req1 = JsonRpcRequest::new("test", None, Id::Number(1));
        let req2 = JsonRpcRequest::new("test", None, Id::Number(2));

        let batch = vec![
            serde_json::to_value(&req1).unwrap(),
            serde_json::to_value(&req2).unwrap(),
        ];

        let responses = processor
            .process_batch(batch, &router, 1, &sub_manager)
            .await;

        assert_eq!(responses.len(), 2);
    }

    #[tokio::test]
    async fn test_sequential_batch() {
        let mut router = Router::new();
        let handler = from_fn(|_| async { Ok(serde_json::json!({"result": "ok"})) });
        router.register("test", handler);

        let sub_manager = SubscriptionManager::new();
        let processor = BatchProcessor::new(BatchMode::Sequential);

        let req1 = JsonRpcRequest::new("test", None, Id::Number(1));
        let req2 = JsonRpcRequest::new("test", None, Id::Number(2));

        let batch = vec![
            serde_json::to_value(&req1).unwrap(),
            serde_json::to_value(&req2).unwrap(),
        ];

        let responses = processor
            .process_batch(batch, &router, 1, &sub_manager)
            .await;

        assert_eq!(responses.len(), 2);
    }

    #[tokio::test]
    async fn test_batch_with_notification() {
        let mut router = Router::new();
        let handler = from_fn(|_| async { Ok(serde_json::json!({"result": "ok"})) });
        router.register("test", handler);

        let sub_manager = SubscriptionManager::new();
        let processor = BatchProcessor::new(BatchMode::Parallel);

        let req = JsonRpcRequest::new("test", None, Id::Number(1));
        let notif = jrow_core::JsonRpcNotification::new("test", None);

        let batch = vec![
            serde_json::to_value(&req).unwrap(),
            serde_json::to_value(&notif).unwrap(),
        ];

        let responses = processor
            .process_batch(batch, &router, 1, &sub_manager)
            .await;

        // Only one response (notification doesn't get a response)
        assert_eq!(responses.len(), 1);
    }

    #[tokio::test]
    async fn test_batch_size_limit_within() {
        let mut router = Router::new();
        let handler = from_fn(|_| async { Ok(serde_json::json!({"result": "ok"})) });
        router.register("test", handler);

        let sub_manager = SubscriptionManager::new();
        // Set limit to 3
        let processor = BatchProcessor::with_limit(BatchMode::Parallel, Some(3));

        let req1 = JsonRpcRequest::new("test", None, Id::Number(1));
        let req2 = JsonRpcRequest::new("test", None, Id::Number(2));

        let batch = vec![
            serde_json::to_value(&req1).unwrap(),
            serde_json::to_value(&req2).unwrap(),
        ];

        let responses = processor
            .process_batch(batch, &router, 1, &sub_manager)
            .await;

        // Should process normally - 2 requests, both succeed
        assert_eq!(responses.len(), 2);
        assert!(responses[0].error.is_none());
        assert!(responses[1].error.is_none());
    }

    #[tokio::test]
    async fn test_batch_size_limit_exceeded() {
        let mut router = Router::new();
        let handler = from_fn(|_| async { Ok(serde_json::json!({"result": "ok"})) });
        router.register("test", handler);

        let sub_manager = SubscriptionManager::new();
        // Set limit to 2
        let processor = BatchProcessor::with_limit(BatchMode::Parallel, Some(2));

        let req1 = JsonRpcRequest::new("test", None, Id::Number(1));
        let req2 = JsonRpcRequest::new("test", None, Id::Number(2));
        let req3 = JsonRpcRequest::new("test", None, Id::Number(3));

        let batch = vec![
            serde_json::to_value(&req1).unwrap(),
            serde_json::to_value(&req2).unwrap(),
            serde_json::to_value(&req3).unwrap(),
        ];

        let responses = processor
            .process_batch(batch, &router, 1, &sub_manager)
            .await;

        // Should return single error response
        assert_eq!(responses.len(), 1);
        assert!(responses[0].error.is_some());
        let error = responses[0].error.as_ref().unwrap();
        assert_eq!(error.code, -32600);
        assert!(error.message.contains("Batch size limit exceeded"));
    }

    #[tokio::test]
    async fn test_batch_size_unlimited() {
        let mut router = Router::new();
        let handler = from_fn(|_| async { Ok(serde_json::json!({"result": "ok"})) });
        router.register("test", handler);

        let sub_manager = SubscriptionManager::new();
        // No limit
        let processor = BatchProcessor::with_limit(BatchMode::Parallel, None);

        // Create a large batch (100 requests)
        let batch: Vec<_> = (0..100)
            .map(|i| {
                let req = JsonRpcRequest::new("test", None, Id::Number(i));
                serde_json::to_value(&req).unwrap()
            })
            .collect();

        let responses = processor
            .process_batch(batch, &router, 1, &sub_manager)
            .await;

        // Should process all 100 requests
        assert_eq!(responses.len(), 100);
    }

    #[tokio::test]
    async fn test_batch_size_limit_edge_case_one() {
        let mut router = Router::new();
        let handler = from_fn(|_| async { Ok(serde_json::json!({"result": "ok"})) });
        router.register("test", handler);

        let sub_manager = SubscriptionManager::new();
        // Limit of 1
        let processor = BatchProcessor::with_limit(BatchMode::Parallel, Some(1));

        let req1 = JsonRpcRequest::new("test", None, Id::Number(1));
        let batch = vec![serde_json::to_value(&req1).unwrap()];

        let responses = processor
            .process_batch(batch, &router, 1, &sub_manager)
            .await;

        // Should process the single request
        assert_eq!(responses.len(), 1);
        assert!(responses[0].error.is_none());

        // Now try with 2 requests
        let req2 = JsonRpcRequest::new("test", None, Id::Number(2));
        let batch = vec![
            serde_json::to_value(&req1).unwrap(),
            serde_json::to_value(&req2).unwrap(),
        ];

        let responses = processor
            .process_batch(batch, &router, 1, &sub_manager)
            .await;

        // Should return error
        assert_eq!(responses.len(), 1);
        assert!(responses[0].error.is_some());
    }
}
