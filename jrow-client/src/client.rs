//! JSON-RPC client implementation over WebSocket
//!
//! This module provides the main `JrowClient` type, which manages the WebSocket
//! connection and provides methods for making requests, subscribing to topics,
//! and handling notifications.
//!
//! # Client Lifecycle
//!
//! 1. **Connect**: Establish WebSocket connection
//! 2. **Use**: Make requests, subscribe to topics
//! 3. **Reconnect** (optional): Automatically reconnect on failure
//! 4. **Close**: Drop the client to close the connection
//!
//! # Cloning
//!
//! `JrowClient` is cheaply cloneable using `Arc` internally. All clones
//! share the same connection and state. This allows you to use the client
//! from multiple tasks.
//!
//! # Thread Safety
//!
//! The client is fully thread-safe and can be shared across tasks without
//! additional synchronization.

use crate::{connection_state::ConnectionManager, request::RequestManager, NotificationHandler};
use futures::{SinkExt, StreamExt};
use jrow_core::{codec, Error, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

/// Pending request to be sent after reconnection
#[derive(Clone)]
pub(crate) struct PendingRequest {
    #[allow(dead_code)]
    method: String,
    #[allow(dead_code)]
    params: serde_json::Value,
}

/// Persistent subscription info for reconnection
#[derive(Clone)]
pub(crate) struct PersistentSubscriptionInfo {
    pub(crate) subscription_id: String,
    pub(crate) topic: String,
}

/// JSON-RPC client over WebSocket
#[derive(Clone)]
pub struct JrowClient {
    /// WebSocket sender
    pub(crate) sender: Arc<
        Mutex<
            futures::stream::SplitSink<
                WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
                Message,
            >,
        >,
    >,
    /// Request manager for tracking pending requests
    pub(crate) request_manager: RequestManager,
    /// Notification handler for incoming notifications
    pub(crate) notification_handler: NotificationHandler,
    /// Set of subscribed topics
    pub(crate) subscribed_topics: Arc<Mutex<HashSet<String>>>,
    /// Persistent subscriptions for auto-resume on reconnect
    pub(crate) persistent_subscriptions: Arc<Mutex<Vec<PersistentSubscriptionInfo>>>,
    /// Connection manager for reconnection
    pub(crate) connection_manager: Option<Arc<ConnectionManager>>,
    /// Pending requests to be sent after reconnection (reserved for future use)
    #[allow(dead_code)]
    pub(crate) pending_requests: Arc<RwLock<Vec<PendingRequest>>>,
    /// Metrics for observability
    pub(crate) metrics: Option<Arc<crate::ClientMetrics>>,
}

impl JrowClient {
    /// Connect to a JSON-RPC server over WebSocket (without reconnection)
    /// For reconnection support, use `ClientBuilder::new(url).with_reconnect(...).connect()`
    #[tracing::instrument(skip(url), fields(url = url))]
    pub async fn connect(url: &str) -> Result<Self> {
        tracing::info!("Connecting to server");
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| Error::WebSocket(e.to_string()))?;

        let (sender, receiver) = ws_stream.split();
        let sender = Arc::new(Mutex::new(sender));

        let request_manager = RequestManager::new();
        let notification_handler = NotificationHandler::new();
        let subscribed_topics = Arc::new(Mutex::new(HashSet::new()));

        let persistent_subscriptions = Arc::new(Mutex::new(Vec::new()));

        let client = Self {
            sender: sender.clone(),
            request_manager: request_manager.clone(),
            notification_handler: notification_handler.clone(),
            subscribed_topics: subscribed_topics.clone(),
            persistent_subscriptions: persistent_subscriptions.clone(),
            connection_manager: None,
            pending_requests: Arc::new(RwLock::new(Vec::new())),
            metrics: None,
        };

        tracing::info!("Connected successfully");

        // Spawn a task to handle incoming messages
        tokio::spawn(Self::receive_loop_with_reconnect(
            receiver,
            request_manager,
            notification_handler,
            sender,
            None,
            subscribed_topics,
            persistent_subscriptions,
            url.to_string(),
            None,
        ));

        Ok(client)
    }

    /// Get the current connection state (if reconnection is enabled)
    pub async fn connection_state(&self) -> Option<crate::ConnectionState> {
        if let Some(ref cm) = self.connection_manager {
            Some(cm.state().await)
        } else {
            None
        }
    }

    /// Check if the client is currently connected
    pub async fn is_connected(&self) -> bool {
        if let Some(ref cm) = self.connection_manager {
            matches!(
                cm.state().await,
                crate::ConnectionState::Connected
            )
        } else {
            // Without connection manager, assume connected if sender is not closed
            true
        }
    }

    /// Send a JSON-RPC request and wait for the response
    #[tracing::instrument(skip(self, params), fields(method = %method.as_ref()))]
    pub async fn request<P, R>(&self, method: impl Into<String> + AsRef<str>, params: P) -> Result<R>
    where
        P: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let start = std::time::Instant::now();
        let method = method.into();
        let params_value =
            serde_json::to_value(params).map_err(|e| Error::Serialization(e.to_string()))?;

        let id = self.request_manager.next_id().await;
        let request = JsonRpcRequest::new(method.clone(), Some(params_value), id.clone());

        // Register the pending request before sending
        let rx = self.request_manager.register(id.clone()).await;

        // Send the request
        let request_text = codec::encode_request(&request)?;
        self.sender
            .lock()
            .await
            .send(Message::Text(request_text))
            .await
            .map_err(|e| Error::WebSocket(e.to_string()))?;

        tracing::debug!("Request sent, waiting for response");

        // Wait for the response
        let response = rx
            .await
            .map_err(|_| Error::Internal("Request channel closed".to_string()))??;

        let duration = start.elapsed().as_secs_f64();

        // Check if the response is an error
        if let Some(error) = response.error {
            if let Some(ref m) = self.metrics {
                m.record_request(&method, "error", duration);
                m.record_error("json_rpc");
            }
            tracing::error!(method = %method, error = ?error, "Request failed");
            return Err(Error::JsonRpc(error));
        }

        // Deserialize the result
        let result = response
            .result
            .ok_or_else(|| Error::Internal("Response missing result".to_string()))?;

        let deserialized: R = serde_json::from_value(result).map_err(|e| Error::Serialization(e.to_string()))?;

        // Record success metrics
        if let Some(ref m) = self.metrics {
            m.record_request(&method, "success", duration);
        }

        tracing::debug!(method = %method, duration_secs = duration, "Request completed successfully");
        Ok(deserialized)
    }

    /// Send a JSON-RPC notification (no response expected)
    pub async fn notify<P>(&self, method: impl Into<String>, params: P) -> Result<()>
    where
        P: serde::Serialize,
    {
        let method = method.into();
        let params_value =
            serde_json::to_value(params).map_err(|e| Error::Serialization(e.to_string()))?;

        let notification = JsonRpcNotification::new(method, Some(params_value));
        let notification_text = codec::encode_notification(&notification)?;

        self.sender
            .lock()
            .await
            .send(Message::Text(notification_text))
            .await
            .map_err(|e| Error::WebSocket(e.to_string()))?;

        Ok(())
    }

    /// Register a handler for incoming notifications
    pub async fn on_notification<F, Fut>(&self, method: impl Into<String>, handler: F)
    where
        F: Fn(JsonRpcNotification) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.notification_handler.register(method, handler).await;
    }

    /// Get the notification handler
    pub fn notification_handler(&self) -> &NotificationHandler {
        &self.notification_handler
    }

    /// Subscribe to a topic and register a handler for messages on that topic
    pub async fn subscribe<F, Fut>(&self, topic: impl Into<String>, handler: F) -> Result<()>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let topic = topic.into();

        // Register the notification handler first
        self.notification_handler
            .register(topic.clone(), move |notif| {
                let handler_future = if let Some(params) = notif.params {
                    handler(params)
                } else {
                    handler(serde_json::Value::Null)
                };
                Box::pin(handler_future)
            })
            .await;

        // Send subscribe RPC request
        #[derive(Serialize)]
        struct SubscribeParams {
            topic: String,
        }

        #[derive(Deserialize)]
        struct SubscribeResult {
            subscribed: bool,
            #[allow(dead_code)]
            topic: String,
        }

        let result: SubscribeResult = self
            .request(
                "rpc.subscribe",
                SubscribeParams {
                    topic: topic.clone(),
                },
            )
            .await?;

        if result.subscribed {
            // Track subscription locally
            self.subscribed_topics.lock().await.insert(topic);
            Ok(())
        } else {
            Err(Error::Internal("Failed to subscribe".to_string()))
        }
    }

    /// Unsubscribe from a topic
    pub async fn unsubscribe(&self, topic: impl Into<String>) -> Result<()> {
        let topic = topic.into();

        // Send unsubscribe RPC request
        #[derive(Serialize)]
        struct UnsubscribeParams {
            topic: String,
        }

        #[derive(Deserialize)]
        struct UnsubscribeResult {
            #[allow(dead_code)]
            unsubscribed: bool,
            #[allow(dead_code)]
            topic: String,
        }

        let _result: UnsubscribeResult = self
            .request(
                "rpc.unsubscribe",
                UnsubscribeParams {
                    topic: topic.clone(),
                },
            )
            .await?;

        // Remove local handler and tracking
        self.notification_handler.unregister(&topic).await;
        self.subscribed_topics.lock().await.remove(&topic);

        Ok(())
    }

    /// Subscribe to multiple topics at once using a batch request
    pub async fn subscribe_batch<F, Fut>(&self, topics: Vec<(String, F)>) -> Result<()>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static + Clone,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        if topics.is_empty() {
            return Ok(());
        }

        // Register all notification handlers first
        for (topic, handler) in &topics {
            let handler_clone = handler.clone();
            self.notification_handler
                .register(topic.clone(), move |notif| {
                    let handler_future = if let Some(params) = notif.params {
                        handler_clone(params)
                    } else {
                        handler_clone(serde_json::Value::Null)
                    };
                    Box::pin(handler_future)
                })
                .await;
        }

        // Build batch request
        let mut batch = crate::BatchRequest::new();
        let topic_names: Vec<String> = topics.iter().map(|(t, _)| t.clone()).collect();

        for topic in &topic_names {
            #[derive(Serialize)]
            struct SubscribeParams {
                topic: String,
            }
            batch.add_request(
                "rpc.subscribe",
                SubscribeParams {
                    topic: topic.clone(),
                },
            );
        }

        // Send batch and verify all succeeded
        let responses = self.batch(batch).await?;

        if !responses.all_success() {
            // Rollback: unregister all handlers
            for topic in &topic_names {
                self.notification_handler.unregister(topic).await;
            }
            return Err(Error::Internal(
                "Failed to subscribe to all topics".to_string(),
            ));
        }

        // Track all subscriptions locally
        let mut subscribed = self.subscribed_topics.lock().await;
        for topic in topic_names {
            subscribed.insert(topic);
        }

        Ok(())
    }

    /// Unsubscribe from multiple topics at once using a batch request
    pub async fn unsubscribe_batch(&self, topics: Vec<String>) -> Result<()> {
        if topics.is_empty() {
            return Ok(());
        }

        // Build batch request
        let mut batch = crate::BatchRequest::new();

        for topic in &topics {
            #[derive(Serialize)]
            struct UnsubscribeParams {
                topic: String,
            }
            batch.add_request(
                "rpc.unsubscribe",
                UnsubscribeParams {
                    topic: topic.clone(),
                },
            );
        }

        // Send batch
        let _responses = self.batch(batch).await?;

        // Remove local handlers and tracking (best effort, even if some failed)
        for topic in &topics {
            self.notification_handler.unregister(topic).await;
        }

        let mut subscribed = self.subscribed_topics.lock().await;
        for topic in topics {
            subscribed.remove(&topic);
        }

        Ok(())
    }

    /// Get list of currently subscribed topics
    pub async fn subscriptions(&self) -> Vec<String> {
        self.subscribed_topics
            .lock()
            .await
            .iter()
            .cloned()
            .collect()
    }

    /// Subscribe to a topic with persistent tracking (exactly-once delivery)
    /// 
    /// This method provides:
    /// - Persistent message storage on the server
    /// - Automatic resume from last acknowledged position on reconnection
    /// - Exactly-once delivery semantics with manual acknowledgment
    /// 
    /// Returns the last acknowledged sequence ID
    pub async fn subscribe_persistent<F, Fut>(
        &self,
        subscription_id: impl Into<String>,
        topic: impl Into<String>,
        handler: F,
    ) -> Result<u64>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let subscription_id = subscription_id.into();
        let topic = topic.into();

        // Register the notification handler for this topic
        self.notification_handler
            .register(topic.clone(), move |notif| {
                let handler_future = if let Some(params) = notif.params {
                    handler(params)
                } else {
                    handler(serde_json::Value::Null)
                };
                Box::pin(handler_future)
            })
            .await;

        // Send subscribe_persistent RPC request
        #[derive(Serialize)]
        struct SubscribePersistentParams {
            subscription_id: String,
            topic: String,
        }

        #[derive(Deserialize)]
        struct SubscribePersistentResult {
            subscribed: bool,
            subscription_id: String,
            #[allow(dead_code)]
            topic: String,
            resumed_from_seq: u64,
            #[allow(dead_code)]
            undelivered_count: usize,
        }

        let result: SubscribePersistentResult = self
            .request(
                "rpc.subscribe_persistent",
                SubscribePersistentParams {
                    subscription_id: subscription_id.clone(),
                    topic: topic.clone(),
                },
            )
            .await?;

        if result.subscribed {
            // Track subscription locally
            self.subscribed_topics.lock().await.insert(topic.clone());
            
            // Track persistent subscription for auto-resume on reconnect
            self.persistent_subscriptions.lock().await.push(PersistentSubscriptionInfo {
                subscription_id,
                topic,
            });
            
            Ok(result.resumed_from_seq)
        } else {
            Err(Error::Internal("Failed to subscribe persistently".to_string()))
        }
    }

    /// Acknowledge a persistent message after successful processing
    /// 
    /// This advances the subscription position so the message won't be redelivered.
    /// 
    /// **Note**: This method spawns the acknowledgment in a background task to avoid
    /// blocking the notification handler. Acknowledgments are fire-and-forget operations.
    /// If you need to know when the ack completes, use `ack_persistent_await` instead.
    pub fn ack_persistent(
        &self,
        subscription_id: impl Into<String>,
        sequence_id: u64,
    ) {
        let client = self.clone();
        let subscription_id = subscription_id.into();
        
        tokio::spawn(async move {
            if let Err(e) = client.ack_persistent_await(subscription_id, sequence_id).await {
                tracing::warn!(error = %e, "Failed to acknowledge persistent message");
            }
        });
    }

    /// Acknowledge a persistent message and await the result
    /// 
    /// This is the awaitable version of `ack_persistent`. Use this if you need to
    /// know when the acknowledgment completes or handle errors synchronously.
    /// 
    /// **Warning**: Do not call this directly from within a notification handler
    /// as it may cause deadlocks. Use `ack_persistent` instead in handlers.
    pub async fn ack_persistent_await(
        &self,
        subscription_id: impl Into<String>,
        sequence_id: u64,
    ) -> Result<()> {
        let subscription_id = subscription_id.into();

        #[derive(Serialize)]
        struct AckPersistentParams {
            subscription_id: String,
            sequence_id: u64,
        }

        #[derive(Deserialize)]
        struct AckPersistentResult {
            acknowledged: bool,
            #[allow(dead_code)]
            subscription_id: String,
            #[allow(dead_code)]
            sequence_id: u64,
        }

        let result: AckPersistentResult = self
            .request(
                "rpc.ack_persistent",
                AckPersistentParams {
                    subscription_id,
                    sequence_id,
                },
            )
            .await?;

        if result.acknowledged {
            Ok(())
        } else {
            Err(Error::Internal("Failed to acknowledge message".to_string()))
        }
    }

    /// Unsubscribe from a persistent subscription
    /// 
    /// Note: This removes the subscription from active connections but keeps
    /// the subscription state in storage, allowing you to resume later
    pub async fn unsubscribe_persistent(&self, subscription_id: impl Into<String>) -> Result<()> {
        let subscription_id = subscription_id.into();

        #[derive(Serialize)]
        struct UnsubscribePersistentParams {
            subscription_id: String,
        }

        #[derive(Deserialize)]
        struct UnsubscribePersistentResult {
            #[allow(dead_code)]
            unsubscribed: bool,
            #[allow(dead_code)]
            subscription_id: String,
        }

        let _result: UnsubscribePersistentResult = self
            .request(
                "rpc.unsubscribe_persistent",
                UnsubscribePersistentParams {
                    subscription_id: subscription_id.clone(),
                },
            )
            .await?;

        // Remove from local tracking
        self.persistent_subscriptions
            .lock()
            .await
            .retain(|info| info.subscription_id != subscription_id);

        Ok(())
    }

    /// Subscribe to multiple persistent subscriptions at once using a batch request
    /// 
    /// This method provides the same guarantees as `subscribe_persistent` but for multiple
    /// subscriptions at once, reducing network overhead.
    /// 
    /// Returns a vector of (subscription_id, resumed_sequence_id) pairs in the same order as input.
    pub async fn subscribe_persistent_batch<F, Fut>(
        &self,
        subscriptions: Vec<(String, String, F)>, // (subscription_id, topic, handler)
    ) -> Result<Vec<(String, u64)>>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static + Clone,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        use serde::{Deserialize, Serialize};

        if subscriptions.is_empty() {
            return Ok(Vec::new());
        }

        #[derive(Serialize)]
        struct SubscribePersistentItem {
            subscription_id: String,
            topic: String,
        }

        #[derive(Deserialize)]
        struct SubscribeResult {
            subscription_id: String,
            topic: String,
            success: bool,
            resumed_from_seq: u64,
            #[allow(dead_code)]
            undelivered_count: usize,
            error: Option<String>,
        }

        // Register all notification handlers first
        for (subscription_id, topic, handler) in &subscriptions {
            let handler_clone = handler.clone();
            self.notification_handler
                .register(topic.clone(), move |notif| {
                    let handler_future = if let Some(params) = notif.params {
                        handler_clone(params)
                    } else {
                        handler_clone(serde_json::Value::Null)
                    };
                    Box::pin(handler_future)
                })
                .await;

            // Track persistent subscription
            self.persistent_subscriptions.lock().await.push(PersistentSubscriptionInfo {
                subscription_id: subscription_id.clone(),
                topic: topic.clone(),
            });
        }

        // Build batch request
        let items: Vec<SubscribePersistentItem> = subscriptions
            .iter()
            .map(|(sub_id, topic, _)| SubscribePersistentItem {
                subscription_id: sub_id.clone(),
                topic: topic.clone(),
            })
            .collect();

        // Send batch subscribe request
        let results: Vec<SubscribeResult> = self
            .request("rpc.subscribe_persistent_batch", items)
            .await?;

        // Check for any failures and collect results
        let mut resumed_seqs = Vec::with_capacity(results.len());
        for result in results {
            if !result.success {
                tracing::warn!(
                    subscription_id = %result.subscription_id,
                    error = ?result.error,
                    "Failed to subscribe to persistent subscription in batch"
                );
            }
            resumed_seqs.push((result.subscription_id, result.resumed_from_seq));
        }

        Ok(resumed_seqs)
    }

    /// Acknowledge multiple persistent messages at once
    /// 
    /// This is a fire-and-forget operation that spawns acknowledgments in the background.
    /// Use `ack_persistent_batch_await` if you need to wait for completion.
    pub fn ack_persistent_batch(&self, acknowledgments: Vec<(String, u64)>) {
        let client = self.clone();
        
        tokio::spawn(async move {
            if let Err(e) = client.ack_persistent_batch_await(acknowledgments).await {
                tracing::warn!(error = %e, "Failed to acknowledge persistent messages in batch");
            }
        });
    }

    /// Acknowledge multiple persistent messages and await the result
    /// 
    /// Returns a vector of (subscription_id, sequence_id, success) tuples indicating
    /// which acknowledgments succeeded.
    /// 
    /// **Warning**: Do not call this directly from within a notification handler
    /// as it may cause deadlocks. Use `ack_persistent_batch` instead in handlers.
    pub async fn ack_persistent_batch_await(
        &self,
        acknowledgments: Vec<(String, u64)>,
    ) -> Result<Vec<(String, u64, bool)>> {
        use serde::{Deserialize, Serialize};

        if acknowledgments.is_empty() {
            return Ok(Vec::new());
        }

        #[derive(Serialize)]
        struct AckPersistentItem {
            subscription_id: String,
            sequence_id: u64,
        }

        #[derive(Deserialize)]
        struct AckResult {
            subscription_id: String,
            sequence_id: u64,
            acknowledged: bool,
            #[allow(dead_code)]
            error: Option<String>,
        }

        let items: Vec<AckPersistentItem> = acknowledgments
            .iter()
            .map(|(sub_id, seq_id)| AckPersistentItem {
                subscription_id: sub_id.clone(),
                sequence_id: *seq_id,
            })
            .collect();

        let results: Vec<AckResult> = self
            .request("rpc.ack_persistent_batch", items)
            .await?;

        Ok(results
            .into_iter()
            .map(|r| (r.subscription_id, r.sequence_id, r.acknowledged))
            .collect())
    }

    /// Unsubscribe from multiple persistent subscriptions at once
    /// 
    /// Note: This removes the subscriptions from active connections but keeps
    /// the subscription state in storage, allowing you to resume later.
    pub async fn unsubscribe_persistent_batch(
        &self,
        subscription_ids: Vec<String>,
    ) -> Result<()> {
        use serde::Deserialize;

        if subscription_ids.is_empty() {
            return Ok(());
        }

        #[derive(Deserialize)]
        struct UnsubscribeResult {
            subscription_id: String,
            unsubscribed: bool,
            #[allow(dead_code)]
            error: Option<String>,
        }

        let results: Vec<UnsubscribeResult> = self
            .request("rpc.unsubscribe_persistent_batch", subscription_ids.clone())
            .await?;

        // Log any failures
        for result in results {
            if !result.unsubscribed {
                tracing::warn!(
                    subscription_id = %result.subscription_id,
                    "Failed to unsubscribe from persistent subscription in batch"
                );
            }
        }

        // Remove from local tracking
        let mut persistent_subs = self.persistent_subscriptions.lock().await;
        persistent_subs.retain(|info| !subscription_ids.contains(&info.subscription_id));

        Ok(())
    }

    /// Send a batch request
    #[tracing::instrument(skip(self, batch), fields(batch_size = batch.requests().len() + batch.notifications().len()))]
    pub async fn batch(&self, batch: crate::BatchRequest) -> Result<crate::BatchResponse> {
        if batch.is_empty() {
            return Err(Error::InvalidRequest("Batch cannot be empty".to_string()));
        }

        let batch_size = (batch.requests().len() + batch.notifications().len()) as u64;

        // Build the batch message array
        let mut batch_messages: Vec<serde_json::Value> = Vec::new();

        // Add all requests
        for request in batch.requests() {
            batch_messages.push(
                serde_json::to_value(request).map_err(|e| Error::Serialization(e.to_string()))?,
            );
        }

        // Add all notifications
        for notification in batch.notifications() {
            batch_messages.push(
                serde_json::to_value(notification)
                    .map_err(|e| Error::Serialization(e.to_string()))?,
            );
        }

        // Register all request IDs for tracking
        let request_ids = batch.request_ids();
        let mut receivers = Vec::new();
        for id in &request_ids {
            let rx = self.request_manager.register(id.clone()).await;
            receivers.push((id.clone(), rx));
        }

        // Encode and send the batch
        let batch_text = serde_json::to_string(&batch_messages)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        self.sender
            .lock()
            .await
            .send(Message::Text(batch_text))
            .await
            .map_err(|e| Error::WebSocket(e.to_string()))?;

        tracing::debug!("Batch request sent, waiting for responses");

        // Wait for all responses
        let mut responses = Vec::new();
        for (_id, rx) in receivers {
            match rx.await {
                Ok(Ok(response)) => responses.push(response),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(Error::Internal("Request channel closed".to_string())),
            }
        }

        // Record metrics
        if let Some(ref m) = self.metrics {
            m.record_batch(batch_size);
        }

        tracing::debug!(response_count = responses.len(), "Batch request completed");
        Ok(crate::BatchResponse::new(responses))
    }

    /// Wrapper for receive loop that handles reconnection
    pub(crate) async fn receive_loop_with_reconnect(
        mut receiver: futures::stream::SplitStream<
            WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        >,
        request_manager: RequestManager,
        notification_handler: NotificationHandler,
        sender: Arc<
            Mutex<
                futures::stream::SplitSink<
                    WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
                    Message,
                >,
            >,
        >,
        connection_manager: Option<Arc<ConnectionManager>>,
        subscribed_topics: Arc<Mutex<HashSet<String>>>,
        persistent_subscriptions: Arc<Mutex<Vec<PersistentSubscriptionInfo>>>,
        url: String,
        metrics: Option<Arc<crate::ClientMetrics>>,
    ) {
        loop {
            // Process messages until disconnection
            while let Some(message) = receiver.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Err(e) =
                            Self::handle_message(&text, &request_manager, &notification_handler, &metrics)
                                .await
                        {
                            tracing::error!(error = %e, "Error handling message");
                            if let Some(ref m) = metrics {
                                m.record_error("message_handling");
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("Connection closed by server");
                        break;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "WebSocket error");
                        if let Some(ref m) = metrics {
                            m.record_error("websocket");
                        }
                        break;
                    }
                    _ => {} // Ignore other message types
                }
            }

            // Connection lost - attempt reconnection if enabled
            if let Some(ref cm) = connection_manager {
                cm.disconnected().await;
                cm.start_reconnecting().await.ok();
                
                if let Some(ref m) = metrics {
                    m.update_connection_state(3); // Reconnecting
                }

                // Reconnection loop
                loop {
                    let delay = cm.next_reconnect_delay().await;

                    match delay {
                        Some(duration) => {
                            let attempt = match cm.state().await {
                                crate::ConnectionState::Reconnecting { attempt } => attempt,
                                _ => 0,
                            };
                            
                            tracing::info!(
                                delay_secs = duration.as_secs_f64(),
                                attempt = attempt,
                                "Reconnecting"
                            );
                            
                            if let Some(ref m) = metrics {
                                m.record_reconnection_attempt();
                            }
                            
                            tokio::time::sleep(duration).await;

                            // Attempt to connect
                            match connect_async(&url).await {
                                Ok((ws_stream, _)) => {
                                    tracing::info!("Reconnected successfully");
                                    let (new_sender, new_receiver) = ws_stream.split();

                                    // Update the sender
                                    *sender.lock().await = new_sender;

                                    // Mark as connected
                                    cm.connected().await;
                                    
                                    if let Some(ref m) = metrics {
                                        m.update_connection_state(2); // Connected
                                        m.record_reconnection_success();
                                    }

                                    // Resubscribe to all regular topics
                                    let topics: Vec<String> =
                                        subscribed_topics.lock().await.iter().cloned().collect();
                                    for topic in topics {
                                        tracing::info!(topic = %topic, "Resubscribing to regular topic");
                                        #[derive(Serialize)]
                                        struct SubscribeParams {
                                            topic: String,
                                        }

                                        let id = request_manager.next_id().await;
                                        let request = JsonRpcRequest::new(
                                            "rpc.subscribe".to_string(),
                                            Some(
                                                serde_json::to_value(SubscribeParams {
                                                    topic: topic.clone(),
                                                })
                                                .unwrap(),
                                            ),
                                            id.clone(),
                                        );

                                        if let Ok(request_text) = codec::encode_request(&request) {
                                            let _ = sender
                                                .lock()
                                                .await
                                                .send(Message::Text(request_text))
                                                .await;
                                        }
                                    }

                                    // Resume persistent subscriptions
                                    let persistent_subs: Vec<PersistentSubscriptionInfo> = 
                                        persistent_subscriptions.lock().await.clone();
                                    for sub_info in persistent_subs {
                                        tracing::info!(
                                            subscription_id = %sub_info.subscription_id,
                                            topic = %sub_info.topic,
                                            "Resuming persistent subscription"
                                        );
                                        
                                        #[derive(Serialize)]
                                        struct SubscribePersistentParams {
                                            subscription_id: String,
                                            topic: String,
                                        }

                                        let id = request_manager.next_id().await;
                                        let request = JsonRpcRequest::new(
                                            "rpc.subscribe_persistent".to_string(),
                                            Some(
                                                serde_json::to_value(SubscribePersistentParams {
                                                    subscription_id: sub_info.subscription_id.clone(),
                                                    topic: sub_info.topic.clone(),
                                                })
                                                .unwrap(),
                                            ),
                                            id.clone(),
                                        );

                                        if let Ok(request_text) = codec::encode_request(&request) {
                                            let _ = sender
                                                .lock()
                                                .await
                                                .send(Message::Text(request_text))
                                                .await;
                                        }
                                    }

                                    // Set the new receiver and continue outer loop
                                    receiver = new_receiver;
                                    break;
                                }
                                Err(e) => {
                                    tracing::warn!(error = %e, "Reconnection failed");
                                    if let Some(ref m) = metrics {
                                        m.record_error("reconnection");
                                    }
                                    // Continue loop to try again
                                }
                            }
                        }
                        None => {
                            tracing::error!("Reconnection abandoned (max attempts reached)");
                            if let Some(ref m) = metrics {
                                m.update_connection_state(4); // Failed
                            }
                            request_manager.fail_all(Error::ConnectionClosed).await;
                            return;
                        }
                    }
                }
            } else {
                // No reconnection enabled, fail all pending requests and exit
                tracing::info!("No reconnection enabled, closing client");
                if let Some(ref m) = metrics {
                    m.update_connection_state(0); // Disconnected
                }
                request_manager.fail_all(Error::ConnectionClosed).await;
                break;
            }
        }
    }

    /// Handle a single incoming message
    async fn handle_message(
        text: &str,
        request_manager: &RequestManager,
        notification_handler: &NotificationHandler,
        metrics: &Option<Arc<crate::ClientMetrics>>,
    ) -> Result<()> {
        let message = codec::decode(text)?;

        match message {
            JsonRpcMessage::Response(response) => {
                let id = response.id.clone();
                request_manager.complete(&id, response).await;
            }
            JsonRpcMessage::Notification(notification) => {
                if let Some(ref m) = metrics {
                    m.record_notification(&notification.method);
                }
                tracing::debug!(method = %notification.method, "Notification received");
                notification_handler.handle(notification).await;
            }
            JsonRpcMessage::Request(_) => {
                // Clients don't typically receive requests
                tracing::warn!("Received unexpected request message");
            }
            JsonRpcMessage::Batch(values) => {
                // Decode batch responses
                tracing::debug!(batch_size = values.len(), "Batch response received");
                for value in values {
                    if let Ok(JsonRpcMessage::Response(response)) = serde_json::from_value(value) {
                        let id = response.id.clone();
                        request_manager.complete(&id, response).await;
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require a running server
    // Unit tests are limited to what we can do without network

    #[test]
    fn test_client_creation() {
        // This is a placeholder - full testing requires a server
        assert!(true);
    }
}
