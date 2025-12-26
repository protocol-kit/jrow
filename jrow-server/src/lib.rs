//! JSON-RPC 2.0 server implementation over WebSocket
//!
//! This crate provides a production-ready JSON-RPC 2.0 server that communicates
//! over WebSocket connections. It includes advanced features like pub/sub,
//! pattern matching, persistent subscriptions, batch processing, and middleware.
//!
//! # Core Features
//!
//! - **WebSocket Transport**: Full-duplex communication using async WebSockets
//! - **Method Routing**: Register handlers for JSON-RPC methods
//! - **Pub/Sub**: Built-in support for topic subscriptions and notifications
//! - **Pattern Matching**: NATS-style wildcard subscriptions (`*` and `>`)
//! - **Batch Processing**: Handle multiple requests in a single message
//! - **Middleware**: Request/response interceptors for cross-cutting concerns
//! - **Persistence**: Durable subscriptions with message replay
//! - **Observability**: OpenTelemetry integration for traces and metrics
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use jrow_server::{JrowServer, from_typed_fn};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! struct AddParams { a: i32, b: i32 }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let addr: std::net::SocketAddr = "127.0.0.1:8080".parse()?;
//!     let server = JrowServer::builder()
//!         .bind(addr)
//!         .handler("add", from_typed_fn(|p: AddParams| async move {
//!             Ok(p.a + p.b)
//!         }))
//!         .build()
//!         .await?;
//!     
//!     server.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! The server uses an actor-like model where each connection runs in its own task:
//!
//! - **Main task**: Accepts incoming TCP connections
//! - **Connection tasks**: Handle WebSocket upgrade and message routing
//! - **Handler execution**: Each request spawns async handler execution
//!
//! This design provides:
//! - **Isolation**: Connection failures don't affect other connections
//! - **Concurrency**: Multiple requests processed simultaneously
//! - **Backpressure**: Slow clients don't block fast ones
//!
//! # Pub/Sub Pattern
//!
//! The server provides built-in pub/sub via special RPC methods:
//!
//! - `rpc.subscribe` - Subscribe to a topic or pattern
//! - `rpc.unsubscribe` - Unsubscribe from a topic
//! - Server-to-client notifications for published messages
//!
//! Publishers use `server.publish(topic, data)` to broadcast to subscribers.
//!
//! # Persistence
//!
//! Enable persistent subscriptions for durable message delivery:
//!
//! ```rust,no_run
//! use jrow_server::JrowServer;
//! use std::time::Duration;
//!
//! # async fn example() -> jrow_core::Result<()> {
//! let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
//! let server = JrowServer::builder()
//!     .bind(addr)
//!     .with_persistent_storage("./data/jrow.db")
//!     .subscription_timeout(Duration::from_secs(3600))
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! Persistent subscriptions survive disconnects and replay missed messages.

mod batch;
mod builder;
mod connection;
mod filter;
mod handler;
mod metrics;
mod middleware;
mod nats_pattern;
mod persistent_storage;
mod persistent_subscription;
mod retention;
mod retention_task;
mod router;
mod subscription;

pub use batch::{BatchMode, BatchProcessor};
pub use builder::ServerBuilder;
pub use filter::{FilteredSubscriptionManager, TopicFilter};
pub use handler::{from_fn, from_typed_fn, Handler, HandlerResult};
pub use metrics::ServerMetrics;
pub use middleware::{
    LoggingMiddleware, MetricsMiddleware, Middleware, MiddlewareAction, MiddlewareChain,
    MiddlewareContext, SyncMiddleware, TracingMiddleware,
};
pub use nats_pattern::{NatsPattern, PatternError, Token};
pub use persistent_storage::{PersistentMessage, PersistentStorage, SubscriptionState, TopicMetadata};
pub use persistent_subscription::PersistentSubscriptionManager;
pub use retention::RetentionPolicy;
pub use router::{Router, RouterBuilder};
pub use subscription::SubscriptionManager;

use connection::Connection;
use jrow_core::{Error, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// Registry of active connections
///
/// This type maps connection IDs to connection handles, allowing the server
/// to send notifications to specific connections or broadcast to all connections.
///
/// It's wrapped in Arc<Mutex> for thread-safe shared access across connection tasks.
pub type ConnectionRegistry = Arc<Mutex<HashMap<u64, Connection>>>;

/// JSON-RPC 2.0 server over WebSocket
///
/// This is the main server struct that manages WebSocket connections, routes
/// JSON-RPC requests to handlers, and coordinates pub/sub functionality.
///
/// # Lifecycle
///
/// 1. **Build**: Create server using `JrowServer::builder()`
/// 2. **Run**: Call `server.run().await` to start accepting connections
/// 3. **Publish**: Use `server.publish()` to send notifications to subscribers
/// 4. **Shutdown**: Drop the server or use graceful shutdown mechanisms
///
/// # Concurrency Model
///
/// The server spawns independent tasks for each connection. Shared state
/// (router, subscriptions, etc.) is protected by appropriate synchronization
/// primitives (Arc, Mutex, etc.) to allow safe concurrent access.
///
/// # Examples
///
/// See module-level documentation for basic usage examples.
pub struct JrowServer {
    /// TCP listener for accepting incoming connections
    listener: TcpListener,
    /// Router that dispatches requests to handler functions
    router: Router,
    /// Manages exact-match topic subscriptions
    subscription_manager: SubscriptionManager,
    /// Manages pattern-based (wildcard) subscriptions
    filtered_subscription_manager: Arc<Mutex<FilteredSubscriptionManager>>,
    /// Registry of all active connections for broadcasting
    connection_registry: ConnectionRegistry,
    /// Processor for batch JSON-RPC requests
    batch_processor: BatchProcessor,
    /// Optional OpenTelemetry metrics collector
    metrics: Option<Arc<ServerMetrics>>,
    /// Optional persistent storage for durable subscriptions
    persistent_storage: Option<Arc<PersistentStorage>>,
    /// Optional manager for persistent subscriptions
    persistent_sub_manager: Option<Arc<PersistentSubscriptionManager>>,
    /// Channel to signal shutdown to the retention task
    retention_shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl JrowServer {
    /// Create a new server builder
    ///
    /// This is the entry point for constructing a server. Use the builder
    /// pattern to configure the server before calling `build()`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use jrow_server::JrowServer;
    ///
    /// # async fn example() -> jrow_core::Result<()> {
    /// let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// let server = JrowServer::builder()
    ///     .bind(addr)
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Run the server and accept connections
    ///
    /// This method starts the main server loop that:
    /// 1. Accepts incoming TCP connections
    /// 2. Upgrades them to WebSocket
    /// 3. Spawns a task for each connection to handle messages
    ///
    /// This method runs indefinitely until an error occurs or the server
    /// is shut down externally.
    ///
    /// # Concurrency
    ///
    /// Each accepted connection is handled in its own Tokio task, allowing
    /// the server to handle multiple connections concurrently without blocking.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The TCP listener fails to accept a connection
    /// - Other system-level errors occur
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use jrow_server::JrowServer;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let addr: std::net::SocketAddr = "127.0.0.1:8080".parse()?;
    ///     let server = JrowServer::builder()
    ///         .bind(addr)
    ///         .build()
    ///         .await?;
    ///     
    ///     server.run().await?;
    ///     Ok(())
    /// }
    /// ```
    #[tracing::instrument(skip(self), name = "server.run")]
    pub async fn run(&self) -> Result<()> {
        tracing::info!("Starting JROW server");
        // Counter for assigning unique IDs to each connection
        let conn_counter = AtomicU64::new(0);

        loop {
            let (stream, addr) = self
                .listener
                .accept()
                .await
                .map_err(|e| jrow_core::Error::Io(e.to_string()))?;
            let conn_id = conn_counter.fetch_add(1, Ordering::SeqCst);
            let router = self.router.clone();
            let sub_manager = self.subscription_manager.clone();
            let filtered_sub_manager = Arc::clone(&self.filtered_subscription_manager);
            let conn_registry = Arc::clone(&self.connection_registry);
            let batch_processor = self.batch_processor.clone();
            let metrics = self.metrics.clone();
            let persistent_storage = self.persistent_storage.clone();
            let persistent_sub_manager = self.persistent_sub_manager.clone();

            tracing::info!(conn_id = conn_id, addr = %addr, "New connection accepted");

            // Record connection metrics
            if let Some(ref m) = metrics {
                let active = conn_counter.load(Ordering::SeqCst) as i64;
                m.record_connection(active);
            }

            // Spawn a task to handle the connection
            tokio::spawn(async move {
                if let Err(e) = connection::handle_connection(
                    stream,
                    conn_id,
                    router,
                    sub_manager,
                    filtered_sub_manager,
                    conn_registry,
                    batch_processor,
                    metrics,
                    persistent_storage,
                    persistent_sub_manager,
                )
                .await
                {
                    tracing::error!(conn_id = conn_id, error = %e, "Connection error");
                }
            });
        }
    }

    /// Publish a message to all subscribers of a topic
    ///
    /// This method broadcasts a notification to all connections subscribed to the
    /// given topic, including both:
    /// - **Exact matches**: Connections subscribed to this exact topic
    /// - **Pattern matches**: Connections subscribed to patterns that match this topic
    ///
    /// # How It Works
    ///
    /// 1. Finds all exact topic subscribers
    /// 2. Finds all pattern subscribers whose patterns match the topic
    /// 3. Sends notifications to all matching connections
    /// 4. Records metrics if observability is enabled
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name to publish to (e.g., "users.login")
    /// * `data` - JSON data to include in the notification
    ///
    /// # Returns
    ///
    /// The number of connections that successfully received the notification.
    /// Failed sends (e.g., closed connections) are silently ignored.
    ///
    /// # Pattern Matching
    ///
    /// For pattern subscribers, the notification includes both the actual topic
    /// and the data, wrapped in an object: `{"topic": "...", "data": ...}`
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use jrow_server::JrowServer;
    /// # async fn example(server: &JrowServer) -> jrow_core::Result<()> {
    /// use serde_json::json;
    ///
    /// let subscriber_count = server.publish(
    ///     "users.login",
    ///     json!({"user_id": 123, "timestamp": "2024-01-01T00:00:00Z"})
    /// ).await?;
    ///
    /// println!("Notified {} subscribers", subscriber_count);
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(self, data), fields(topic = %topic.as_ref()))]
    pub async fn publish(
        &self,
        topic: impl Into<String> + AsRef<str>,
        data: serde_json::Value,
    ) -> Result<usize> {
        let topic = topic.into();
        
        // Get exact subscribers
        let exact_subscribers = self.subscription_manager.get_subscribers(&topic).await;
        
        // Get pattern-based subscribers with their patterns
        let filtered_subs = self.filtered_subscription_manager.lock().await;
        let pattern_subscribers = filtered_subs.get_subscribers_with_patterns(&topic);
        drop(filtered_subs);
        
        let conn_registry = self.connection_registry.lock().await;

        let mut sent_count = 0;
        
        // Send to exact subscribers (original behavior)
        for conn_id in exact_subscribers {
            if let Some(conn) = conn_registry.get(&conn_id) {
                if conn.notify(&topic, Some(data.clone())).is_ok() {
                    sent_count += 1;
                }
            }
        }

        // Send to pattern subscribers (send to pattern, include actual topic in data)
        for (conn_id, pattern) in pattern_subscribers {
            if let Some(conn) = conn_registry.get(&conn_id) {
                // Wrap data to include the actual topic for pattern subscriptions
                let notification_data = serde_json::json!({
                    "topic": topic,
                    "data": data.clone(),
                });
                
                // Send notification to the pattern, not the actual topic
                if conn.notify(&pattern, Some(notification_data)).is_ok() {
                    sent_count += 1;
                }
            }
        }

        // Record metrics
        if let Some(ref m) = self.metrics {
            m.record_publish(&topic);
        }

        tracing::debug!(topic = %topic, sent_count = sent_count, "Message published");
        Ok(sent_count)
    }

    /// Publish messages to multiple topics at once
    ///
    /// Returns a vector of (topic, subscriber_count) pairs in the same order as input.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use jrow_server::JrowServer;
    /// # async fn example(server: JrowServer) -> Result<(), Box<dyn std::error::Error>> {
    /// let messages = vec![
    ///     ("news".to_string(), serde_json::json!({"title": "Breaking news"})),
    ///     ("alerts".to_string(), serde_json::json!({"level": "warning"})),
    /// ];
    ///
    /// let results = server.publish_batch(messages).await?;
    /// for (topic, count) in results {
    ///     println!("Published to '{}': {} subscribers", topic, count);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(self, messages), fields(batch_size = messages.len()))]
    pub async fn publish_batch(
        &self,
        messages: Vec<(String, serde_json::Value)>,
    ) -> Result<Vec<(String, usize)>> {
        let mut results = Vec::with_capacity(messages.len());

        // Lock the connection registry once for all publishes
        let conn_registry = self.connection_registry.lock().await;
        let filtered_subs = self.filtered_subscription_manager.lock().await;

        for (topic, data) in messages {
            // Get exact subscribers
            let exact_subscribers = self.subscription_manager.get_subscribers(&topic).await;
            
            // Get pattern-based subscribers with their patterns
            let pattern_subscribers = filtered_subs.get_subscribers_with_patterns(&topic);

            let mut sent_count = 0;
            
            // Send to exact subscribers (original behavior)
            for conn_id in exact_subscribers {
                if let Some(conn) = conn_registry.get(&conn_id) {
                    if conn.notify(&topic, Some(data.clone())).is_ok() {
                        sent_count += 1;
                    }
                }
            }
            
            // Send to pattern subscribers (send to pattern, include actual topic in data)
            for (conn_id, pattern) in pattern_subscribers {
                if let Some(conn) = conn_registry.get(&conn_id) {
                    // Wrap data to include the actual topic for pattern subscriptions
                    let notification_data = serde_json::json!({
                        "topic": topic,
                        "data": data.clone(),
                    });
                    
                    // Send notification to the pattern, not the actual topic
                    if conn.notify(&pattern, Some(notification_data)).is_ok() {
                        sent_count += 1;
                    }
                }
            }

            // Record metrics for each topic
            if let Some(ref m) = self.metrics {
                m.record_publish(&topic);
            }

            results.push((topic.clone(), sent_count));
        }

        tracing::debug!(batch_size = results.len(), "Batch publish completed");
        Ok(results)
    }

    /// Get the subscription manager
    ///
    /// Returns a reference to the subscription manager for advanced use cases
    /// like querying subscription state or manually managing subscriptions.
    ///
    /// Most applications don't need to call this directly; use `publish()` instead.
    pub fn subscription_manager(&self) -> &SubscriptionManager {
        &self.subscription_manager
    }

    /// Publish a message to persistent storage and active persistent subscribers
    ///
    /// This is the durable counterpart to `publish()`. It:
    /// 1. Stores the message persistently in the database
    /// 2. Assigns it a monotonically increasing sequence ID
    /// 3. Delivers it to currently active persistent subscribers
    ///
    /// # Persistence Guarantees
    ///
    /// Once this method returns Ok, the message is:
    /// - **Durably stored**: Survives server restarts
    /// - **Sequenced**: Has a unique, ordered sequence ID
    /// - **Replayable**: Subscribers can retrieve it after reconnecting
    ///
    /// # Requirements
    ///
    /// Persistent storage must be configured via
    /// `ServerBuilder::with_persistent_storage()` or this method returns an error.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name (can be a pattern like "events.*")
    /// * `data` - JSON data to store and deliver
    ///
    /// # Returns
    ///
    /// - `Ok(sequence_id)`: The assigned sequence ID
    /// - `Err(...)`: If persistent storage is not configured or storage fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use jrow_server::JrowServer;
    /// # async fn example(server: &JrowServer) -> jrow_core::Result<()> {
    /// use serde_json::json;
    ///
    /// let seq_id = server.publish_persistent(
    ///     "orders.created",
    ///     json!({"order_id": 12345, "amount": 99.99})
    /// ).await?;
    ///
    /// println!("Stored with sequence ID: {}", seq_id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// - `publish()` for non-persistent pub/sub
    /// - `PersistentStorage` for advanced persistence operations
    #[tracing::instrument(skip(self, data), fields(topic = %topic.as_ref()))]
    pub async fn publish_persistent(
        &self,
        topic: impl Into<String> + AsRef<str>,
        data: serde_json::Value,
    ) -> Result<u64> {
        let topic = topic.into();
        
        let (storage, sub_manager) = match (&self.persistent_storage, &self.persistent_sub_manager) {
            (Some(s), Some(m)) => (s, m),
            _ => return Err(Error::Internal(
                "Persistent storage not configured. Use ServerBuilder::with_persistent_storage()".to_string()
            )),
        };
        
        // Store message and get sequence ID
        let sequence_id = storage.store_message(&topic, data.clone()).await?;
        
        tracing::debug!(
            topic = %topic,
            sequence_id = sequence_id,
            "Message stored persistently"
        );
        
        // Find active persistent subscribers whose patterns match this topic
        let matching_subs = sub_manager.get_matching_subscriptions(&topic).await;
        let conn_registry = self.connection_registry.lock().await;
        
        let mut delivered_count = 0;
        for (subscription_id, conn_id) in matching_subs {
            if let Some(conn) = conn_registry.get(&conn_id) {
                // Create persistent message notification
                let notification_data = serde_json::json!({
                    "sequence_id": sequence_id,
                    "data": data.clone(),
                });
                
                if conn.notify(&topic, Some(notification_data)).is_ok() {
                    delivered_count += 1;
                    tracing::trace!(
                        subscription_id = %subscription_id,
                        conn_id = conn_id,
                        sequence_id = sequence_id,
                        topic = %topic,
                        "Delivered persistent message"
                    );
                }
            }
        }
        
        // Record metrics
        if let Some(ref m) = self.metrics {
            m.record_publish(&topic);
        }
        
        tracing::debug!(
            topic = %topic,
            sequence_id = sequence_id,
            delivered_count = delivered_count,
            "Persistent message published"
        );
        
        Ok(sequence_id)
    }

    /// Get the persistent storage (if configured)
    ///
    /// Returns `Some(&Arc<PersistentStorage>)` if persistent storage was enabled
    /// via `ServerBuilder::with_persistent_storage()`, otherwise `None`.
    ///
    /// Use this for advanced persistence operations like querying message history
    /// or managing retention policies.
    pub fn persistent_storage(&self) -> Option<&Arc<PersistentStorage>> {
        self.persistent_storage.as_ref()
    }

    /// Get the persistent subscription manager (if configured)
    ///
    /// Returns the persistent subscription manager if persistent storage is enabled.
    /// Use this for advanced subscription management like querying subscription state.
    pub fn persistent_sub_manager(&self) -> Option<&Arc<PersistentSubscriptionManager>> {
        self.persistent_sub_manager.as_ref()
    }

    /// Get the local address the server is listening on
    ///
    /// This is useful to discover the actual bound port when using port 0
    /// (which lets the OS choose an available port).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use jrow_server::JrowServer;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse()?;  // OS chooses port
    /// let server = JrowServer::builder()
    ///     .bind(bind_addr)
    ///     .build()
    ///     .await?;
    ///
    /// let addr = server.local_addr()?;
    /// println!("Server listening on {}", addr);
    /// # Ok(())
    /// # }
    /// ```
    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }
}

impl Drop for JrowServer {
    /// Clean up server resources on drop
    ///
    /// This implementation ensures graceful shutdown of background tasks:
    /// - Signals the retention enforcement task to stop
    /// - Allows background tasks to finish cleanup
    ///
    /// Note: Active connections will be forcibly closed when the server is dropped.
    /// For graceful shutdown, ensure all connection tasks complete before dropping.
    fn drop(&mut self) {
        // Signal retention task to shutdown if it's running
        if let Some(tx) = &self.retention_shutdown_tx {
            // Sending true signals the task to stop
            // We ignore send errors (task may have already stopped)
            let _ = tx.send(true);
        }
    }
}
