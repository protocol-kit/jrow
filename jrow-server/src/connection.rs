//! WebSocket connection management for JSON-RPC server
//!
//! This module handles the lifecycle of individual WebSocket connections,
//! from TCP accept to WebSocket upgrade to message processing and cleanup.
//!
//! # Connection Lifecycle
//!
//! 1. **Accept**: TCP connection accepted by main server loop
//! 2. **Upgrade**: Upgrade to WebSocket protocol
//! 3. **Register**: Add to connection registry
//! 4. **Process**: Handle incoming messages, route to handlers
//! 5. **Cleanup**: Remove from registry, clean up subscriptions
//!
//! # Task Model
//!
//! Each connection spawns two tasks:
//! - **Receive task**: Reads WebSocket messages, processes requests
//! - **Send task**: Writes outgoing messages from a channel
//!
//! This decouples sending from receiving, preventing slow sends from
//! blocking message processing.
//!
//! # Built-in Methods
//!
//! The connection handler implements several built-in JSON-RPC methods:
//! - `rpc.subscribe` - Subscribe to a topic or pattern
//! - `rpc.unsubscribe` - Unsubscribe from a topic
//! - `rpc.subscribe_persistent` - Durable subscription with replay
//! - `rpc.ack_persistent` - Acknowledge persistent message delivery
//!
//! # Error Handling
//!
//! Connection errors (network issues, protocol violations) cause the
//! connection to close. The connection is automatically removed from
//! the registry and all subscriptions are cleaned up.

use crate::router::Router;
use futures::{SinkExt, StreamExt};
use jrow_core::{
    codec, Error, JsonRpcErrorData, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, Result,
};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};

/// Handle for a WebSocket connection
///
/// This handle allows sending notifications to a specific connection.
/// It's lightweight (just an ID and channel sender) and can be cloned
/// to send from multiple places.
///
/// # Cloning
///
/// Cloning a `Connection` handle creates a new sender to the same
/// underlying channel, allowing multiple tasks to send to the same connection.
#[derive(Clone)]
pub struct Connection {
    /// Unique connection ID assigned by the server
    pub id: u64,
    /// Channel sender for outgoing WebSocket messages
    /// Using unbounded channel prevents send tasks from blocking
    tx: mpsc::UnboundedSender<Message>,
}

impl Connection {
    /// Create a new connection handle
    pub fn new(id: u64, tx: mpsc::UnboundedSender<Message>) -> Self {
        Self { id, tx }
    }

    /// Send a notification to the client
    pub fn notify(
        &self,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
    ) -> Result<()> {
        let notification = JsonRpcNotification::new(method, params);
        let msg = codec::encode_notification(&notification)?;
        self.tx
            .send(Message::Text(msg))
            .map_err(|_| Error::ConnectionClosed)?;
        Ok(())
    }

    /// Send a raw message to the client
    #[allow(dead_code)]
    pub fn send_message(&self, msg: Message) -> Result<()> {
        self.tx.send(msg).map_err(|_| Error::ConnectionClosed)?;
        Ok(())
    }
}

/// Handle a single WebSocket connection
#[tracing::instrument(skip(stream, router, sub_manager, filtered_sub_manager, conn_registry, batch_processor, metrics, persistent_storage, persistent_sub_manager), fields(conn_id = conn_id))]
pub async fn handle_connection(
    stream: TcpStream,
    conn_id: u64,
    router: Router,
    sub_manager: crate::SubscriptionManager,
    filtered_sub_manager: std::sync::Arc<tokio::sync::Mutex<crate::FilteredSubscriptionManager>>,
    conn_registry: crate::ConnectionRegistry,
    batch_processor: crate::BatchProcessor,
    metrics: Option<std::sync::Arc<crate::ServerMetrics>>,
    persistent_storage: Option<std::sync::Arc<crate::PersistentStorage>>,
    persistent_sub_manager: Option<std::sync::Arc<crate::PersistentSubscriptionManager>>,
) -> Result<()> {
    tracing::debug!("Upgrading connection to WebSocket");
    // Upgrade to WebSocket
    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| Error::WebSocket(e.to_string()))?;

    // Split the WebSocket stream
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Create a channel for outgoing messages
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Create connection handle
    let conn = Connection::new(conn_id, tx.clone());

    // Register connection in the registry
    {
        let mut registry = conn_registry.lock().await;
        registry.insert(conn_id, conn.clone());
    }

    // Spawn task to forward messages from channel to WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(msg).await {
                tracing::error!(error = %e, "Error sending message");
                break;
            }
        }
    });

    // Handle incoming messages
    let router_clone = router.clone();
    let tx_clone = tx.clone();
    let sub_manager_clone = sub_manager.clone();
    let filtered_sub_manager_clone = filtered_sub_manager.clone();
    let batch_processor_clone = batch_processor.clone();
    let metrics_clone = metrics.clone();
    let persistent_storage_clone = persistent_storage.clone();
    let persistent_sub_manager_clone = persistent_sub_manager.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = handle_message(
                        &text,
                        &router_clone,
                        &tx_clone,
                        conn_id,
                        &sub_manager_clone,
                        &filtered_sub_manager_clone,
                        &batch_processor_clone,
                        &metrics_clone,
                        &persistent_storage_clone,
                        &persistent_sub_manager_clone,
                    )
                    .await
                    {
                        tracing::error!(error = %e, "Error handling message");
                        if let Some(ref m) = metrics_clone {
                            m.record_error("message_handling");
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Connection closed by client");
                    break;
                }
                Ok(_) => {} // Ignore other message types
                Err(e) => {
                    tracing::error!(error = %e, "WebSocket error");
                    if let Some(ref m) = metrics_clone {
                        m.record_error("websocket");
                    }
                    break;
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    // Cleanup: remove connection from registry and all subscriptions
    {
        let mut registry = conn_registry.lock().await;
        registry.remove(&conn_id);
    }
    sub_manager.remove_connection(conn_id).await;
    filtered_sub_manager.lock().await.remove_connection(conn_id);
    
    // Clean up persistent subscriptions
    if let Some(ref psm) = persistent_sub_manager {
        psm.remove_connection(conn_id).await;
    }
    
    // Record disconnection metrics
    if let Some(ref m) = metrics {
        let registry = conn_registry.lock().await;
        let active = registry.len() as i64;
        m.record_disconnection(active);
    }
    
    tracing::info!("Connection cleaned up");

    Ok(())
}

/// Handle a single JSON-RPC message
#[tracing::instrument(skip(text, router, tx, sub_manager, filtered_sub_manager, batch_processor, metrics, persistent_storage, persistent_sub_manager), fields(conn_id = conn_id))]
async fn handle_message(
    text: &str,
    router: &Router,
    tx: &mpsc::UnboundedSender<Message>,
    conn_id: u64,
    sub_manager: &crate::SubscriptionManager,
    filtered_sub_manager: &std::sync::Arc<tokio::sync::Mutex<crate::FilteredSubscriptionManager>>,
    batch_processor: &crate::BatchProcessor,
    metrics: &Option<std::sync::Arc<crate::ServerMetrics>>,
    persistent_storage: &Option<std::sync::Arc<crate::PersistentStorage>>,
    persistent_sub_manager: &Option<std::sync::Arc<crate::PersistentSubscriptionManager>>,
) -> Result<()> {
    let start = std::time::Instant::now();
    let message = codec::decode(text)?;

    match message {
        JsonRpcMessage::Request(request) => {
            let method = request.method.clone();
            let response = process_request(
                request,
                router,
                conn_id,
                sub_manager,
                filtered_sub_manager,
                persistent_storage,
                persistent_sub_manager,
                tx,
            ).await;
            let response_text = codec::encode_response(&response)?;
            // Send response back to client
            tx.send(Message::Text(response_text))
                .map_err(|_| Error::ConnectionClosed)?;
            
            // Record metrics
            if let Some(ref m) = metrics {
                let duration = start.elapsed().as_secs_f64();
                let status = if response.error.is_none() { "success" } else { "error" };
                m.record_request(&method, status, duration);
            }
        }
        JsonRpcMessage::Notification(notification) => {
            // Process notification (no response needed)
            if let Err(e) = process_notification(notification, router, conn_id).await {
                tracing::error!(error = %e, "Error processing notification");
            }
        }
        JsonRpcMessage::Response(_) => {
            // Servers don't typically receive responses
            tracing::warn!("Received unexpected response message");
        }
        JsonRpcMessage::Batch(batch_values) => {
            // Process batch request
            let batch_size = batch_values.len();
            tracing::debug!(batch_size = batch_size, "Processing batch request");
            
            let responses = batch_processor
                .process_batch(batch_values, router, conn_id, sub_manager)
                .await;

            if !responses.is_empty() {
                let response_text = codec::encode_batch_responses(&responses)?;
                tx.send(Message::Text(response_text))
                    .map_err(|_| Error::ConnectionClosed)?;
            }
            
            // Record batch metrics
            if let Some(ref m) = metrics {
                m.record_batch(batch_size as u64, "unknown");
            }
        }
    }

    Ok(())
}

/// Process a JSON-RPC request and return a response (public for batch processor)
/// Note: Batch requests only support exact topic subscriptions, not patterns
pub async fn process_request_for_batch(
    request: JsonRpcRequest,
    router: &Router,
    conn_id: u64,
    sub_manager: &crate::SubscriptionManager,
) -> JsonRpcResponse {
    let id = request.id.clone();
    let method = request.method.as_str();

    // Handle built-in subscription methods (exact topics only in batch mode)
    if method == "rpc.subscribe" {
        return handle_subscribe_exact(request, conn_id, sub_manager).await;
    } else if method == "rpc.unsubscribe" {
        return handle_unsubscribe_exact(request, conn_id, sub_manager).await;
    }

    match router.route_with_conn_id(&request.method, request.params, conn_id).await {
        Ok(result) => JsonRpcResponse::success(result, id),
        Err(Error::MethodNotFound(method)) => {
            JsonRpcResponse::error(JsonRpcErrorData::method_not_found(method), id)
        }
        Err(Error::InvalidParams(msg)) => {
            JsonRpcResponse::error(JsonRpcErrorData::invalid_params(msg), id)
        }
        Err(e) => JsonRpcResponse::error(JsonRpcErrorData::internal_error(e.to_string()), id),
    }
}

/// Handle subscribe request for exact topics only (used in batch mode)
async fn handle_subscribe_exact(
    request: JsonRpcRequest,
    conn_id: u64,
    sub_manager: &crate::SubscriptionManager,
) -> JsonRpcResponse {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct SubscribeParams {
        topic: String,
    }

    let id = request.id.clone();

    let params: SubscribeParams = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(params) => params,
            Err(e) => {
                return JsonRpcResponse::error(JsonRpcErrorData::invalid_params(e.to_string()), id);
            }
        },
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params("Missing 'topic' parameter"),
                id,
            );
        }
    };

    sub_manager.subscribe(conn_id, &params.topic).await;

    JsonRpcResponse::success(
        serde_json::json!({
            "subscribed": true,
            "topic": params.topic,
            "pattern": false
        }),
        id,
    )
}

/// Handle unsubscribe request for exact topics only (used in batch mode)
async fn handle_unsubscribe_exact(
    request: JsonRpcRequest,
    conn_id: u64,
    sub_manager: &crate::SubscriptionManager,
) -> JsonRpcResponse {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct UnsubscribeParams {
        topic: String,
    }

    let id = request.id.clone();

    let params: UnsubscribeParams = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(params) => params,
            Err(e) => {
                return JsonRpcResponse::error(JsonRpcErrorData::invalid_params(e.to_string()), id);
            }
        },
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params("Missing 'topic' parameter"),
                id,
            );
        }
    };

    let was_subscribed = sub_manager.unsubscribe(conn_id, &params.topic).await;

    JsonRpcResponse::success(
        serde_json::json!({
            "unsubscribed": was_subscribed,
            "topic": params.topic
        }),
        id,
    )
}

/// Process a JSON-RPC notification
async fn process_notification(notification: JsonRpcNotification, router: &Router, conn_id: u64) -> Result<()> {
    // Notifications don't return responses, but we still route them
    router
        .route_with_conn_id(&notification.method, notification.params, conn_id)
        .await?;
    Ok(())
}

/// Process a JSON-RPC request and return a response (internal)
async fn process_request(
    request: JsonRpcRequest,
    router: &Router,
    conn_id: u64,
    sub_manager: &crate::SubscriptionManager,
    filtered_sub_manager: &std::sync::Arc<tokio::sync::Mutex<crate::FilteredSubscriptionManager>>,
    persistent_storage: &Option<std::sync::Arc<crate::PersistentStorage>>,
    persistent_sub_manager: &Option<std::sync::Arc<crate::PersistentSubscriptionManager>>,
    tx: &mpsc::UnboundedSender<Message>,
) -> JsonRpcResponse {
    let id = request.id.clone();
    let method = request.method.as_str();

    // Handle built-in subscription methods
    if method == "rpc.subscribe" {
        return handle_subscribe(request, conn_id, sub_manager, filtered_sub_manager).await;
    } else if method == "rpc.unsubscribe" {
        return handle_unsubscribe(request, conn_id, sub_manager, filtered_sub_manager).await;
    } else if method == "rpc.subscribe_persistent" {
        return handle_subscribe_persistent(request, conn_id, persistent_storage, persistent_sub_manager, tx).await;
    } else if method == "rpc.ack_persistent" {
        return handle_ack_persistent(request, conn_id, persistent_sub_manager).await;
    } else if method == "rpc.unsubscribe_persistent" {
        return handle_unsubscribe_persistent(request, conn_id, persistent_sub_manager).await;
    } else if method == "rpc.subscribe_persistent_batch" {
        return handle_subscribe_persistent_batch(request, conn_id, persistent_storage, persistent_sub_manager, tx).await;
    } else if method == "rpc.ack_persistent_batch" {
        return handle_ack_persistent_batch(request, conn_id, persistent_sub_manager).await;
    } else if method == "rpc.unsubscribe_persistent_batch" {
        return handle_unsubscribe_persistent_batch(request, conn_id, persistent_sub_manager).await;
    }

    match router.route_with_conn_id(&request.method, request.params, conn_id).await {
        Ok(result) => JsonRpcResponse::success(result, id),
        Err(Error::MethodNotFound(method)) => {
            JsonRpcResponse::error(JsonRpcErrorData::method_not_found(method), id)
        }
        Err(Error::InvalidParams(msg)) => {
            JsonRpcResponse::error(JsonRpcErrorData::invalid_params(msg), id)
        }
        Err(e) => JsonRpcResponse::error(JsonRpcErrorData::internal_error(e.to_string()), id),
    }
}

/// Handle subscribe request
async fn handle_subscribe(
    request: JsonRpcRequest,
    conn_id: u64,
    sub_manager: &crate::SubscriptionManager,
    filtered_sub_manager: &std::sync::Arc<tokio::sync::Mutex<crate::FilteredSubscriptionManager>>,
) -> JsonRpcResponse {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct SubscribeParams {
        topic: String,
    }

    let id = request.id.clone();

    // Parse parameters
    let params: SubscribeParams = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(params) => params,
            Err(e) => {
                return JsonRpcResponse::error(JsonRpcErrorData::invalid_params(e.to_string()), id);
            }
        },
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params("Missing 'topic' parameter"),
                id,
            );
        }
    };

    // Check if topic is a NATS pattern (contains * or >)
    let is_pattern = params.topic.contains('*') || params.topic.contains('>');

    if is_pattern {
        // Use filtered subscription manager for patterns
        match crate::TopicFilter::new(&params.topic) {
            Ok(filter) => {
                filtered_sub_manager.lock().await.subscribe(conn_id, filter);
            }
            Err(e) => {
                return JsonRpcResponse::error(
                    JsonRpcErrorData::invalid_params(format!("Invalid pattern: {}", e)),
                    id,
                );
            }
        }
    } else {
        // Use regular subscription manager for exact topics
        sub_manager.subscribe(conn_id, &params.topic).await;
    }

    // Return success
    JsonRpcResponse::success(
        serde_json::json!({
            "subscribed": true,
            "topic": params.topic,
            "pattern": is_pattern
        }),
        id,
    )
}

/// Handle unsubscribe request
async fn handle_unsubscribe(
    request: JsonRpcRequest,
    conn_id: u64,
    sub_manager: &crate::SubscriptionManager,
    filtered_sub_manager: &std::sync::Arc<tokio::sync::Mutex<crate::FilteredSubscriptionManager>>,
) -> JsonRpcResponse {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct UnsubscribeParams {
        topic: String,
    }

    let id = request.id.clone();

    // Parse parameters
    let params: UnsubscribeParams = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(params) => params,
            Err(e) => {
                return JsonRpcResponse::error(JsonRpcErrorData::invalid_params(e.to_string()), id);
            }
        },
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params("Missing 'topic' parameter"),
                id,
            );
        }
    };

    // Try both managers
    let was_subscribed_exact = sub_manager.unsubscribe(conn_id, &params.topic).await;
    let was_subscribed_pattern = filtered_sub_manager
        .lock()
        .await
        .unsubscribe(conn_id, &params.topic);
    
    let was_subscribed = was_subscribed_exact || was_subscribed_pattern;

    // Return success
    JsonRpcResponse::success(
        serde_json::json!({
            "unsubscribed": was_subscribed,
            "topic": params.topic
        }),
        id,
    )
}

/// Handle persistent subscribe request
async fn handle_subscribe_persistent(
    request: JsonRpcRequest,
    conn_id: u64,
    persistent_storage: &Option<std::sync::Arc<crate::PersistentStorage>>,
    persistent_sub_manager: &Option<std::sync::Arc<crate::PersistentSubscriptionManager>>,
    tx: &mpsc::UnboundedSender<Message>,
) -> JsonRpcResponse {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct SubscribePersistentParams {
        subscription_id: String,
        topic: String,
    }

    let id = request.id.clone();

    // Check if persistent storage is enabled
    let (storage, sub_manager) = match (persistent_storage, persistent_sub_manager) {
        (Some(s), Some(m)) => (s, m),
        _ => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::internal_error("Persistent storage not configured"),
                id,
            );
        }
    };

    // Parse parameters
    let params: SubscribePersistentParams = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(params) => params,
            Err(e) => {
                return JsonRpcResponse::error(JsonRpcErrorData::invalid_params(e.to_string()), id);
            }
        },
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params("Missing 'subscription_id' and 'topic' parameters"),
                id,
            );
        }
    };

    // Register subscription
    let state = match sub_manager
        .register_subscription(params.subscription_id.clone(), params.topic.clone(), conn_id)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::internal_error(format!("Failed to register subscription: {}", e)),
                id,
            );
        }
    };

    // Parse pattern and get undelivered messages (since last ack)
    let pattern = match crate::NatsPattern::new(&params.topic) {
        Ok(p) => p,
        Err(e) => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params(format!("Invalid topic pattern: {}", e)),
                id,
            );
        }
    };
    
    let messages = match storage.get_messages_matching_pattern(&pattern, state.last_ack_seq).await {
        Ok(m) => m,
        Err(e) => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::internal_error(format!("Failed to retrieve messages: {}", e)),
                id,
            );
        }
    };

    tracing::info!(
        subscription_id = %params.subscription_id,
        topic = %params.topic,
        last_ack_seq = state.last_ack_seq,
        undelivered_count = messages.len(),
        "Persistent subscription registered"
    );

    // Save message count before delivering
    let undelivered_count = messages.len();

    // Deliver undelivered messages to the client
    for message in messages {
        // Parse data from JSON string
        let data_value = match message.data_as_value() {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    sequence_id = message.sequence_id,
                    "Failed to parse message data"
                );
                continue;
            }
        };
        
        let notification_data = serde_json::json!({
            "sequence_id": message.sequence_id,
            "topic": message.topic,  // Include actual topic in data
            "data": data_value,
        });
        
        // Send notification to the subscription's topic/pattern, not the message topic
        let notification = JsonRpcNotification::new(&params.topic, Some(notification_data));
        if let Ok(notification_text) = codec::encode_notification(&notification) {
            // Send the notification (ignore errors, client will resume on reconnect)
            let _ = tx.send(Message::Text(notification_text));
            
            tracing::trace!(
                subscription_id = %params.subscription_id,
                sequence_id = message.sequence_id,
                "Delivered backlog message"
            );
        }
    }

    // Return success with state info
    JsonRpcResponse::success(
        serde_json::json!({
            "subscribed": true,
            "subscription_id": params.subscription_id,
            "topic": params.topic,
            "resumed_from_seq": state.last_ack_seq,
            "undelivered_count": undelivered_count,
        }),
        id,
    )
}

/// Handle persistent acknowledgment
async fn handle_ack_persistent(
    request: JsonRpcRequest,
    conn_id: u64,
    persistent_sub_manager: &Option<std::sync::Arc<crate::PersistentSubscriptionManager>>,
) -> JsonRpcResponse {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct AckPersistentParams {
        subscription_id: String,
        sequence_id: u64,
    }

    let id = request.id.clone();

    // Check if persistent storage is enabled
    let sub_manager = match persistent_sub_manager {
        Some(m) => m,
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::internal_error("Persistent storage not configured"),
                id,
            );
        }
    };

    // Parse parameters
    let params: AckPersistentParams = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(params) => params,
            Err(e) => {
                return JsonRpcResponse::error(JsonRpcErrorData::invalid_params(e.to_string()), id);
            }
        },
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params("Missing 'subscription_id' and 'sequence_id' parameters"),
                id,
            );
        }
    };

    // Acknowledge message
    match sub_manager
        .acknowledge_message(&params.subscription_id, params.sequence_id, conn_id)
        .await
    {
        Ok(()) => {
            tracing::trace!(
                subscription_id = %params.subscription_id,
                sequence_id = params.sequence_id,
                "Message acknowledged"
            );
            JsonRpcResponse::success(
                serde_json::json!({
                    "acknowledged": true,
                    "subscription_id": params.subscription_id,
                    "sequence_id": params.sequence_id,
                }),
                id,
            )
        }
        Err(e) => JsonRpcResponse::error(
            JsonRpcErrorData::internal_error(format!("Failed to acknowledge: {}", e)),
            id,
        ),
    }
}

/// Handle persistent unsubscribe
async fn handle_unsubscribe_persistent(
    request: JsonRpcRequest,
    conn_id: u64,
    persistent_sub_manager: &Option<std::sync::Arc<crate::PersistentSubscriptionManager>>,
) -> JsonRpcResponse {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct UnsubscribePersistentParams {
        subscription_id: String,
    }

    let id = request.id.clone();

    // Check if persistent storage is enabled
    let sub_manager = match persistent_sub_manager {
        Some(m) => m,
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::internal_error("Persistent storage not configured"),
                id,
            );
        }
    };

    // Parse parameters
    let params: UnsubscribePersistentParams = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(params) => params,
            Err(e) => {
                return JsonRpcResponse::error(JsonRpcErrorData::invalid_params(e.to_string()), id);
            }
        },
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params("Missing 'subscription_id' parameter"),
                id,
            );
        }
    };

    // Unsubscribe
    match sub_manager.unsubscribe(&params.subscription_id, conn_id).await {
        Ok(unsubscribed) => {
            tracing::info!(
                subscription_id = %params.subscription_id,
                unsubscribed = unsubscribed,
                "Persistent subscription removed"
            );
            JsonRpcResponse::success(
                serde_json::json!({
                    "unsubscribed": unsubscribed,
                    "subscription_id": params.subscription_id,
                }),
                id,
            )
        }
        Err(e) => JsonRpcResponse::error(
            JsonRpcErrorData::internal_error(format!("Failed to unsubscribe: {}", e)),
            id,
        ),
    }
}

/// Handle batch persistent subscription requests
async fn handle_subscribe_persistent_batch(
    request: JsonRpcRequest,
    conn_id: u64,
    persistent_storage: &Option<std::sync::Arc<crate::PersistentStorage>>,
    persistent_sub_manager: &Option<std::sync::Arc<crate::PersistentSubscriptionManager>>,
    tx: &mpsc::UnboundedSender<Message>,
) -> JsonRpcResponse {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct SubscribePersistentItem {
        subscription_id: String,
        topic: String,
    }

    #[derive(Serialize)]
    struct SubscribeResult {
        subscription_id: String,
        topic: String,
        success: bool,
        resumed_from_seq: u64,
        undelivered_count: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let id = request.id.clone();

    // Check if persistent storage is enabled
    let (storage, sub_manager) = match (persistent_storage, persistent_sub_manager) {
        (Some(s), Some(m)) => (s, m),
        _ => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::internal_error("Persistent storage not configured"),
                id,
            );
        }
    };

    // Parse parameters as array
    let items: Vec<SubscribePersistentItem> = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(items) => items,
            Err(e) => {
                return JsonRpcResponse::error(
                    JsonRpcErrorData::invalid_params(format!("Expected array of subscriptions: {}", e)),
                    id,
                );
            }
        },
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params("Missing subscriptions array"),
                id,
            );
        }
    };

    let mut results = Vec::with_capacity(items.len());

    // Process each subscription
    for item in items {
        // Register subscription
        let state = match sub_manager
            .register_subscription(item.subscription_id.clone(), item.topic.clone(), conn_id)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                results.push(SubscribeResult {
                    subscription_id: item.subscription_id,
                    topic: item.topic,
                    success: false,
                    resumed_from_seq: 0,
                    undelivered_count: 0,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };

        // Parse pattern and get undelivered messages
        let pattern = match crate::NatsPattern::new(&item.topic) {
            Ok(p) => p,
            Err(e) => {
                results.push(SubscribeResult {
                    subscription_id: item.subscription_id,
                    topic: item.topic,
                    success: false,
                    resumed_from_seq: state.last_ack_seq,
                    undelivered_count: 0,
                    error: Some(format!("Invalid topic pattern: {}", e)),
                });
                continue;
            }
        };
        
        let messages = match storage.get_messages_matching_pattern(&pattern, state.last_ack_seq).await {
            Ok(m) => m,
            Err(e) => {
                results.push(SubscribeResult {
                    subscription_id: item.subscription_id,
                    topic: item.topic,
                    success: false,
                    resumed_from_seq: state.last_ack_seq,
                    undelivered_count: 0,
                    error: Some(format!("Failed to retrieve messages: {}", e)),
                });
                continue;
            }
        };

        let undelivered_count = messages.len();

        // Deliver undelivered messages to the client
        for message in messages {
            let data_value = match message.data_as_value() {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        sequence_id = message.sequence_id,
                        "Failed to parse message data"
                    );
                    continue;
                }
            };
            
            let notification_data = serde_json::json!({
                "sequence_id": message.sequence_id,
                "topic": message.topic,  // Include actual topic in data
                "data": data_value,
            });
            
            // Send notification to the subscription's topic/pattern, not the message topic
            let notification = JsonRpcNotification::new(&item.topic, Some(notification_data));
            if let Ok(notification_text) = codec::encode_notification(&notification) {
                let _ = tx.send(Message::Text(notification_text));
                
                tracing::trace!(
                    subscription_id = %item.subscription_id,
                    sequence_id = message.sequence_id,
                    "Delivered backlog message"
                );
            }
        }

        tracing::info!(
            subscription_id = %item.subscription_id,
            topic = %item.topic,
            last_ack_seq = state.last_ack_seq,
            undelivered_count = undelivered_count,
            "Persistent subscription registered (batch)"
        );

        results.push(SubscribeResult {
            subscription_id: item.subscription_id,
            topic: item.topic,
            success: true,
            resumed_from_seq: state.last_ack_seq,
            undelivered_count,
            error: None,
        });
    }

    JsonRpcResponse::success(serde_json::json!(results), id)
}

/// Handle batch persistent acknowledgments
async fn handle_ack_persistent_batch(
    request: JsonRpcRequest,
    conn_id: u64,
    persistent_sub_manager: &Option<std::sync::Arc<crate::PersistentSubscriptionManager>>,
) -> JsonRpcResponse {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct AckPersistentItem {
        subscription_id: String,
        sequence_id: u64,
    }

    #[derive(Serialize)]
    struct AckResult {
        subscription_id: String,
        sequence_id: u64,
        acknowledged: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let id = request.id.clone();

    // Check if persistent storage is enabled
    let sub_manager = match persistent_sub_manager {
        Some(m) => m,
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::internal_error("Persistent storage not configured"),
                id,
            );
        }
    };

    // Parse parameters as array
    let items: Vec<AckPersistentItem> = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(items) => items,
            Err(e) => {
                return JsonRpcResponse::error(
                    JsonRpcErrorData::invalid_params(format!("Expected array of acknowledgments: {}", e)),
                    id,
                );
            }
        },
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params("Missing acknowledgments array"),
                id,
            );
        }
    };

    let mut results = Vec::with_capacity(items.len());

    // Process each acknowledgment
    for item in items {
        match sub_manager
            .acknowledge_message(&item.subscription_id, item.sequence_id, conn_id)
            .await
        {
            Ok(()) => {
                tracing::trace!(
                    subscription_id = %item.subscription_id,
                    sequence_id = item.sequence_id,
                    "Message acknowledged (batch)"
                );
                results.push(AckResult {
                    subscription_id: item.subscription_id,
                    sequence_id: item.sequence_id,
                    acknowledged: true,
                    error: None,
                });
            }
            Err(e) => {
                tracing::warn!(
                    subscription_id = %item.subscription_id,
                    sequence_id = item.sequence_id,
                    error = %e,
                    "Failed to acknowledge message (batch)"
                );
                results.push(AckResult {
                    subscription_id: item.subscription_id,
                    sequence_id: item.sequence_id,
                    acknowledged: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    JsonRpcResponse::success(serde_json::json!(results), id)
}

/// Handle batch persistent unsubscribe requests
async fn handle_unsubscribe_persistent_batch(
    request: JsonRpcRequest,
    conn_id: u64,
    persistent_sub_manager: &Option<std::sync::Arc<crate::PersistentSubscriptionManager>>,
) -> JsonRpcResponse {
    use serde::Serialize;

    #[derive(Serialize)]
    struct UnsubscribeResult {
        subscription_id: String,
        unsubscribed: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let id = request.id.clone();

    // Check if persistent storage is enabled
    let sub_manager = match persistent_sub_manager {
        Some(m) => m,
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::internal_error("Persistent storage not configured"),
                id,
            );
        }
    };

    // Parse parameters as array of subscription IDs
    let subscription_ids: Vec<String> = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(ids) => ids,
            Err(e) => {
                return JsonRpcResponse::error(
                    JsonRpcErrorData::invalid_params(format!("Expected array of subscription IDs: {}", e)),
                    id,
                );
            }
        },
        None => {
            return JsonRpcResponse::error(
                JsonRpcErrorData::invalid_params("Missing subscription IDs array"),
                id,
            );
        }
    };

    let mut results = Vec::with_capacity(subscription_ids.len());

    // Process each unsubscribe
    for subscription_id in subscription_ids {
        match sub_manager.unsubscribe(&subscription_id, conn_id).await {
            Ok(unsubscribed) => {
                tracing::info!(
                    subscription_id = %subscription_id,
                    unsubscribed = unsubscribed,
                    "Persistent subscription removed (batch)"
                );
                results.push(UnsubscribeResult {
                    subscription_id,
                    unsubscribed,
                    error: None,
                });
            }
            Err(e) => {
                tracing::warn!(
                    subscription_id = %subscription_id,
                    error = %e,
                    "Failed to unsubscribe (batch)"
                );
                results.push(UnsubscribeResult {
                    subscription_id,
                    unsubscribed: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    JsonRpcResponse::success(serde_json::json!(results), id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::from_fn;

    #[tokio::test]
    async fn test_process_request() {
        let mut router = Router::new();
        let handler = from_fn(|_| async { Ok(serde_json::json!({"result": 42})) });
        router.register("test", handler);
        let sub_manager = crate::SubscriptionManager::new();
        let filtered_sub_manager = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::FilteredSubscriptionManager::new(),
        ));

        let (tx, _rx) = mpsc::unbounded_channel();
        let request = JsonRpcRequest::new("test", None, jrow_core::Id::Number(1));
        let response = process_request(request, &router, 1, &sub_manager, &filtered_sub_manager, &None, &None, &tx).await;

        assert!(response.is_success());
        assert_eq!(response.result, Some(serde_json::json!({"result": 42})));
    }

    #[tokio::test]
    async fn test_process_request_method_not_found() {
        let router = Router::new();
        let sub_manager = crate::SubscriptionManager::new();
        let filtered_sub_manager = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::FilteredSubscriptionManager::new(),
        ));
        let (tx, _rx) = mpsc::unbounded_channel();
        let request = JsonRpcRequest::new("unknown", None, jrow_core::Id::Number(1));
        let response = process_request(request, &router, 1, &sub_manager, &filtered_sub_manager, &None, &None, &tx).await;

        assert!(response.is_error());
        assert_eq!(response.error.as_ref().unwrap().code, -32601);
    }

    #[tokio::test]
    async fn test_subscribe_request() {
        let router = Router::new();
        let sub_manager = crate::SubscriptionManager::new();
        let filtered_sub_manager = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::FilteredSubscriptionManager::new(),
        ));

        let request = JsonRpcRequest::new(
            "rpc.subscribe",
            Some(serde_json::json!({"topic": "test.topic"})),
            jrow_core::Id::Number(1),
        );

        let (tx, _rx) = mpsc::unbounded_channel();
        let response = process_request(request, &router, 1, &sub_manager, &filtered_sub_manager, &None, &None, &tx).await;

        assert!(response.is_success());
        assert!(response.result.unwrap()["subscribed"].as_bool().unwrap());

        // Verify subscription was registered
        let subscribers = sub_manager.get_subscribers("test.topic").await;
        assert_eq!(subscribers, vec![1]);
    }

    #[tokio::test]
    async fn test_subscribe_pattern() {
        let router = Router::new();
        let sub_manager = crate::SubscriptionManager::new();
        let filtered_sub_manager = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::FilteredSubscriptionManager::new(),
        ));

        // Subscribe to a pattern
        let request = JsonRpcRequest::new(
            "rpc.subscribe",
            Some(serde_json::json!({"topic": "events.*"})),
            jrow_core::Id::Number(1),
        );

        let (tx, _rx) = mpsc::unbounded_channel();
        let response = process_request(request, &router, 1, &sub_manager, &filtered_sub_manager, &None, &None, &tx).await;

        assert!(response.is_success());
        let result = response.result.unwrap();
        assert!(result["subscribed"].as_bool().unwrap());
        assert!(result["pattern"].as_bool().unwrap());

        // Verify pattern subscription was registered
        let subscribers = filtered_sub_manager.lock().await.get_subscribers("events.login");
        assert_eq!(subscribers, vec![1]);
    }

    #[tokio::test]
    async fn test_unsubscribe_request() {
        let router = Router::new();
        let sub_manager = crate::SubscriptionManager::new();
        let filtered_sub_manager = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::FilteredSubscriptionManager::new(),
        ));

        // Subscribe first
        sub_manager.subscribe(1, "test.topic").await;

        // Now unsubscribe
        let request = JsonRpcRequest::new(
            "rpc.unsubscribe",
            Some(serde_json::json!({"topic": "test.topic"})),
            jrow_core::Id::Number(1),
        );

        let (tx, _rx) = mpsc::unbounded_channel();
        let response = process_request(request, &router, 1, &sub_manager, &filtered_sub_manager, &None, &None, &tx).await;

        assert!(response.is_success());
        assert!(response.result.unwrap()["unsubscribed"].as_bool().unwrap());

        // Verify subscription was removed
        let subscribers = sub_manager.get_subscribers("test.topic").await;
        assert!(subscribers.is_empty());
    }
}
