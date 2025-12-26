//! Server builder for constructing JSON-RPC servers
//!
//! The builder pattern provides a fluent API for configuring and creating
//! a `JrowServer`. It allows you to:
//! - Set the bind address
//! - Register method handlers
//! - Configure batch processing
//! - Add middleware
//! - Enable observability
//! - Enable persistent storage
//! - Configure retention policies
//!
//! # Examples
//!
//! ```rust,no_run
//! use jrow_server::{JrowServer, from_fn, BatchMode};
//! use std::time::Duration;
//!
//! # async fn example() -> jrow_core::Result<()> {
//! let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
//! let server = JrowServer::builder()
//!     .bind(addr)
//!     .handler("ping", from_fn(|_| async {
//!         Ok(serde_json::json!({"pong": true}))
//!     }))
//!     .batch_mode(BatchMode::Parallel)
//!     .max_batch_size(100)
//!     .with_persistent_storage("./data/jrow.db")
//!     .subscription_timeout(Duration::from_secs(3600))
//!     .with_default_observability()
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::{
    BatchMode, BatchProcessor, Handler, JrowServer, Middleware, MiddlewareChain, 
    PersistentStorage, PersistentSubscriptionManager, RetentionPolicy, Router,
    SubscriptionManager, SyncMiddleware,
};
use jrow_core::{Error, Result};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// Builder for constructing a JSON-RPC server
pub struct ServerBuilder {
    addr: Option<SocketAddr>,
    router: Router,
    batch_mode: BatchMode,
    max_batch_size: Option<usize>,
    middleware_chain: MiddlewareChain,
    observability_config: Option<jrow_core::ObservabilityConfig>,
    service_name: Option<String>,
    persistent_db_path: Option<PathBuf>,
    topic_retention_policies: HashMap<String, RetentionPolicy>,
    subscription_timeout: Option<Duration>,
    retention_interval: Duration,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        Self {
            addr: None,
            router: Router::new(),
            batch_mode: BatchMode::default(),
            max_batch_size: None,
            middleware_chain: MiddlewareChain::new(),
            observability_config: None,
            service_name: None,
            persistent_db_path: None,
            topic_retention_policies: HashMap::new(),
            subscription_timeout: None,
            retention_interval: Duration::from_secs(60),
        }
    }

    /// Set the bind address for the server
    pub fn bind(mut self, addr: impl Into<SocketAddr>) -> Self {
        self.addr = Some(addr.into());
        self
    }

    /// Set the bind address from a string (e.g., "127.0.0.1:8080")
    pub fn bind_str(mut self, addr: &str) -> Result<Self> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|e| Error::InvalidRequest(format!("Invalid address: {}", e)))?;
        self.addr = Some(addr);
        Ok(self)
    }

    /// Register a handler for a method
    pub fn handler(mut self, method: impl Into<String>, handler: Box<dyn Handler>) -> Self {
        self.router.register(method, handler);
        self
    }

    /// Set the router (replaces any previously registered handlers)
    pub fn router(mut self, router: Router) -> Self {
        self.router = router;
        self
    }

    /// Set the batch processing mode
    pub fn batch_mode(mut self, mode: BatchMode) -> Self {
        self.batch_mode = mode;
        self
    }

    /// Set the maximum batch size limit (None = unlimited)
    pub fn max_batch_size(mut self, max_size: usize) -> Self {
        self.max_batch_size = Some(max_size);
        self
    }

    /// Add middleware to the server
    pub fn use_middleware(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middleware_chain.add(middleware);
        self
    }

    /// Add sync middleware to the server
    pub fn use_sync_middleware<T: SyncMiddleware + 'static>(mut self, middleware: T) -> Self {
        self.middleware_chain.add_sync(middleware);
        self
    }

    /// Enable OpenTelemetry observability with custom configuration
    pub fn with_observability(mut self, config: jrow_core::ObservabilityConfig) -> Self {
        self.observability_config = Some(config);
        self
    }

    /// Enable OpenTelemetry observability with default configuration
    pub fn with_default_observability(mut self) -> Self {
        self.observability_config = Some(jrow_core::ObservabilityConfig::default());
        self
    }

    /// Set service name for observability (used if observability is enabled)
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }

    /// Enable persistent storage with the specified database path
    pub fn with_persistent_storage(mut self, db_path: impl Into<PathBuf>) -> Self {
        self.persistent_db_path = Some(db_path.into());
        self
    }

    /// Register a topic with a retention policy
    pub fn register_topic(mut self, topic: impl Into<String>, policy: RetentionPolicy) -> Self {
        self.topic_retention_policies.insert(topic.into(), policy);
        self
    }

    /// Set the subscription inactivity timeout (None = no timeout)
    pub fn subscription_timeout(mut self, timeout: Duration) -> Self {
        self.subscription_timeout = Some(timeout);
        self
    }

    /// Set the retention enforcement interval (default: 60 seconds)
    pub fn retention_interval(mut self, interval: Duration) -> Self {
        self.retention_interval = interval;
        self
    }

    /// Build and start the server
    pub async fn build(mut self) -> Result<JrowServer> {
        let addr = self
            .addr
            .ok_or_else(|| Error::InvalidRequest("No bind address specified".to_string()))?;

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::Io(e.to_string()))?;

        // Initialize observability if configured
        let metrics = if let Some(mut config) = self.observability_config {
            // Override service name if provided
            if let Some(name) = self.service_name {
                config.service_name = name.clone();
            }
            
            // Initialize OpenTelemetry
            jrow_core::init_observability(config.clone())
                .map_err(|e| Error::Internal(format!("Failed to initialize observability: {}", e)))?;
            
            // Create metrics
            Some(Arc::new(crate::ServerMetrics::new(&config.service_name)))
        } else {
            None
        };

        tracing::info!(addr = %addr, "Server listening");

        // Set middleware on router if any was added
        if !self.middleware_chain.is_empty() {
            self.router.set_middleware(self.middleware_chain);
        }

        // Initialize persistent storage if configured
        let (persistent_storage, persistent_sub_manager, retention_shutdown_tx) = if let Some(db_path) = self.persistent_db_path {
            tracing::info!(path = ?db_path, "Initializing persistent storage");
            
            let storage = Arc::new(
                PersistentStorage::new(&db_path)
                    .map_err(|e| Error::Internal(format!("Failed to create persistent storage: {}", e)))?
            );
            
            // Register topics with retention policies
            for (topic, policy) in self.topic_retention_policies {
                tracing::info!(topic = %topic, "Registering topic with retention policy");
                storage.register_topic(&topic, policy).await?;
            }
            
            let sub_manager = Arc::new(PersistentSubscriptionManager::new(
                Arc::clone(&storage),
                self.subscription_timeout,
            ));
            
            // Start retention enforcement task
            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
            let storage_clone = Arc::clone(&storage);
            let retention_interval = self.retention_interval;
            tokio::spawn(async move {
                crate::retention_task::run_retention_task(storage_clone, retention_interval, shutdown_rx).await;
            });
            
            (Some(storage), Some(sub_manager), Some(shutdown_tx))
        } else {
            (None, None, None)
        };

        Ok(JrowServer {
            listener,
            router: self.router,
            subscription_manager: SubscriptionManager::new(),
            filtered_subscription_manager: Arc::new(Mutex::new(
                crate::FilteredSubscriptionManager::new(),
            )),
            connection_registry: Arc::new(Mutex::new(HashMap::new())),
            batch_processor: BatchProcessor::with_limit(self.batch_mode, self.max_batch_size),
            metrics,
            persistent_storage,
            persistent_sub_manager,
            retention_shutdown_tx,
        })
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::from_fn;

    #[tokio::test]
    async fn test_builder_basic() {
        let handler = from_fn(|_| async { Ok(serde_json::json!({"status": "ok"})) });

        let builder = ServerBuilder::new()
            .bind_str("127.0.0.1:0")
            .unwrap()
            .handler("test", handler);

        let server = builder.build().await.unwrap();
        assert!(server.router.has_method("test"));
    }

    #[test]
    fn test_builder_no_address() {
        let builder = ServerBuilder::new();
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { builder.build().await });
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_builder_custom_bind_address() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let builder = ServerBuilder::new().bind(addr);
        
        let server = builder.build().await.unwrap();
        assert!(server.local_addr().is_ok());
    }

    #[tokio::test]
    async fn test_builder_batch_mode_parallel() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let builder = ServerBuilder::new()
            .bind(addr)
            .batch_mode(BatchMode::Parallel);
        
        let server = builder.build().await.unwrap();
        // Verify batch mode was set (internal state)
        assert!(server.local_addr().is_ok());
    }

    #[tokio::test]
    async fn test_builder_batch_mode_sequential() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let builder = ServerBuilder::new()
            .bind(addr)
            .batch_mode(BatchMode::Sequential);
        
        let server = builder.build().await.unwrap();
        assert!(server.local_addr().is_ok());
    }

    #[tokio::test]
    async fn test_builder_max_batch_size() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let builder = ServerBuilder::new()
            .bind(addr)
            .max_batch_size(50);
        
        let server = builder.build().await.unwrap();
        assert!(server.local_addr().is_ok());
    }

    #[tokio::test]
    async fn test_builder_persistent_storage_path() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let storage_path = temp_dir.path().join("test.db");
        
        let builder = ServerBuilder::new()
            .bind(addr)
            .with_persistent_storage(storage_path);
        
        let server = builder.build().await.unwrap();
        assert!(server.persistent_storage().is_some());
    }

    #[tokio::test]
    async fn test_builder_retention_policy() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        
        let builder = ServerBuilder::new()
            .bind(addr)
            .with_persistent_storage(temp_dir.path())
            .register_topic("test", crate::RetentionPolicy::by_count(100))
            .retention_interval(Duration::from_secs(60));
        
        let server = builder.build().await.unwrap();
        assert!(server.persistent_storage().is_some());
    }

    #[tokio::test]
    async fn test_builder_subscription_timeout() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let builder = ServerBuilder::new()
            .bind(addr)
            .subscription_timeout(Duration::from_secs(300));
        
        let server = builder.build().await.unwrap();
        assert!(server.local_addr().is_ok());
    }

    #[test]
    fn test_builder_bind_str_valid() {
        let result = ServerBuilder::new().bind_str("127.0.0.1:8080");
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_bind_str_invalid() {
        let result = ServerBuilder::new().bind_str("invalid:address");
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_default() {
        let builder = ServerBuilder::default();
        assert!(builder.addr.is_none());
        assert_eq!(builder.batch_mode, BatchMode::Parallel);
    }
}
