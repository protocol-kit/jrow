//! Client builder for configuring reconnection and other options
//!
//! The `ClientBuilder` provides a fluent API for configuring client behavior
//! before connecting. It allows you to:
//! - Enable automatic reconnection with various strategies
//! - Configure observability (OpenTelemetry)
//! - Set service name for telemetry
//!
//! # Examples
//!
//! ```rust,no_run
//! use jrow_client::{ClientBuilder, ExponentialBackoff};
//! use std::time::Duration;
//!
//! # async fn example() -> jrow_core::Result<()> {
//! // With reconnection
//! let client = ClientBuilder::new("ws://localhost:8080")
//!     .with_reconnect(Box::new(ExponentialBackoff::default()))
//!     .connect()
//!     .await?;
//!
//! // With observability
//! let client2 = ClientBuilder::new("ws://localhost:8080")
//!     .with_default_observability()
//!     .service_name("my-client")
//!     .connect()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::{
    connection_state::ConnectionManager, reconnect::ReconnectionStrategy, JrowClient,
    NotificationHandler,
};
use crate::{reconnect::ExponentialBackoff, request::RequestManager};
use futures::StreamExt;
use jrow_core::{Error, Result};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::connect_async;

/// Builder for configuring and creating a JrowClient
pub struct ClientBuilder {
    url: String,
    reconnect_strategy: Option<Box<dyn ReconnectionStrategy>>,
    enable_reconnect: bool,
    observability_config: Option<jrow_core::ObservabilityConfig>,
    service_name: Option<String>,
}

impl ClientBuilder {
    /// Create a new client builder
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            reconnect_strategy: None,
            enable_reconnect: false,
            observability_config: None,
            service_name: None,
        }
    }

    /// Enable automatic reconnection with the given strategy
    pub fn with_reconnect(mut self, strategy: Box<dyn ReconnectionStrategy>) -> Self {
        self.reconnect_strategy = Some(strategy);
        self.enable_reconnect = true;
        self
    }

    /// Enable automatic reconnection with default exponential backoff
    pub fn with_default_reconnect(mut self) -> Self {
        self.reconnect_strategy = Some(Box::new(ExponentialBackoff::default()));
        self.enable_reconnect = true;
        self
    }

    /// Disable automatic reconnection (default)
    pub fn without_reconnect(mut self) -> Self {
        self.enable_reconnect = false;
        self.reconnect_strategy = None;
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

    /// Build and connect the client
    pub async fn connect(self) -> Result<JrowClient> {
        let request_manager = RequestManager::new();
        let notification_handler = NotificationHandler::new();
        let subscribed_topics = Arc::new(Mutex::new(HashSet::new()));
        let persistent_subscriptions = Arc::new(Mutex::new(Vec::new()));

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
            Some(Arc::new(crate::ClientMetrics::new(&config.service_name)))
        } else {
            None
        };

        let connection_manager = if self.enable_reconnect {
            let strategy = self
                .reconnect_strategy
                .unwrap_or_else(|| Box::new(ExponentialBackoff::default()));
            Some(Arc::new(ConnectionManager::new(
                self.url.clone(),
                strategy,
            )))
        } else {
            None
        };

        // Initial connection
        tracing::info!(url = %self.url, "Connecting to server");
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| Error::WebSocket(e.to_string()))?;

        let (sender, receiver) = ws_stream.split();
        let sender = Arc::new(Mutex::new(sender));

        // Mark as connected if using connection manager
        if let Some(ref cm) = connection_manager {
            cm.connected().await;
        }

        // Update metrics
        if let Some(ref m) = metrics {
            m.update_connection_state(2); // Connected
        }

        let client = JrowClient {
            sender: sender.clone(),
            request_manager: request_manager.clone(),
            notification_handler: notification_handler.clone(),
            subscribed_topics: subscribed_topics.clone(),
            persistent_subscriptions: persistent_subscriptions.clone(),
            connection_manager: connection_manager.clone(),
            pending_requests: Arc::new(RwLock::new(Vec::new())),
            metrics: metrics.clone(),
        };

        tracing::info!("Connected successfully");

        // Spawn receive loop
        tokio::spawn(JrowClient::receive_loop_with_reconnect(
            receiver,
            request_manager.clone(),
            notification_handler.clone(),
            sender.clone(),
            connection_manager.clone(),
            subscribed_topics.clone(),
            persistent_subscriptions.clone(),
            self.url.clone(),
            metrics,
        ));

        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconnect::FixedDelay;
    use std::time::Duration;

    #[test]
    fn test_builder_creation() {
        let builder = ClientBuilder::new("ws://localhost:8080");
        assert_eq!(builder.url, "ws://localhost:8080");
        assert!(!builder.enable_reconnect);
        assert!(builder.reconnect_strategy.is_none());
    }

    #[test]
    fn test_builder_with_reconnect() {
        let strategy = Box::new(FixedDelay::new(Duration::from_secs(1)));
        let builder = ClientBuilder::new("ws://localhost:8080").with_reconnect(strategy);
        assert!(builder.enable_reconnect);
        assert!(builder.reconnect_strategy.is_some());
    }

    #[test]
    fn test_builder_with_default_reconnect() {
        let builder = ClientBuilder::new("ws://localhost:8080").with_default_reconnect();
        assert!(builder.enable_reconnect);
        assert!(builder.reconnect_strategy.is_some());
    }

    #[test]
    fn test_builder_without_reconnect() {
        let builder = ClientBuilder::new("ws://localhost:8080")
            .with_default_reconnect()
            .without_reconnect();
        assert!(!builder.enable_reconnect);
    }

    #[test]
    fn test_builder_with_custom_reconnect() {
        let strategy = Box::new(FixedDelay::new(Duration::from_millis(500)).with_max_attempts(10));
        let builder = ClientBuilder::new("ws://localhost:8080").with_reconnect(strategy);
        
        assert!(builder.enable_reconnect);
        assert!(builder.reconnect_strategy.is_some());
    }

    #[test]
    fn test_builder_observability_config() {
        let config = jrow_core::ObservabilityConfig::new("test-client")
            .with_endpoint("http://localhost:4317")
            .with_log_level("debug");
        
        let builder = ClientBuilder::new("ws://localhost:8080").with_observability(config);
        
        assert!(builder.observability_config.is_some());
        let obs_config = builder.observability_config.unwrap();
        assert_eq!(obs_config.service_name, "test-client");
        assert_eq!(obs_config.log_level, "debug");
    }

    #[test]
    fn test_builder_default_observability() {
        let builder = ClientBuilder::new("ws://localhost:8080").with_default_observability();
        
        assert!(builder.observability_config.is_some());
        let obs_config = builder.observability_config.unwrap();
        assert_eq!(obs_config.service_name, "jrow");
    }

    #[test]
    fn test_builder_service_name() {
        let builder = ClientBuilder::new("ws://localhost:8080")
            .service_name("my-service");
        
        assert_eq!(builder.service_name, Some("my-service".to_string()));
    }

    #[test]
    fn test_builder_defaults() {
        let builder = ClientBuilder::new("ws://localhost:8080");
        
        // Verify default values
        assert!(!builder.enable_reconnect);
        assert!(builder.reconnect_strategy.is_none());
        assert!(builder.observability_config.is_none());
        assert!(builder.service_name.is_none());
    }

    #[test]
    fn test_builder_chaining() {
        // Test that all builder methods can be chained
        let builder = ClientBuilder::new("ws://localhost:8080")
            .with_default_reconnect()
            .service_name("test-service")
            .with_default_observability();
        
        assert!(builder.enable_reconnect);
        assert!(builder.reconnect_strategy.is_some());
        assert!(builder.observability_config.is_some());
        assert_eq!(builder.service_name, Some("test-service".to_string()));
    }

    #[test]
    fn test_builder_url_storage() {
        let url = "ws://example.com:9000/path";
        let builder = ClientBuilder::new(url);
        
        assert_eq!(builder.url, url);
    }
}

