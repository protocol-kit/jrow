//! Notification handler for JSON-RPC client
//!
//! JSON-RPC notifications are server-to-client messages that don't expect
//! a response. This module provides a handler registry for dispatching
//! incoming notifications to registered callbacks.
//!
//! # Use Cases
//!
//! Notifications are used for:
//! - **Pub/sub events**: Messages published to subscribed topics
//! - **Server-initiated events**: Status updates, alerts
//! - **Broadcast messages**: Announcements to all clients
//!
//! # Handler Registration
//!
//! Handlers are async functions that receive the notification and process
//! it. Multiple topics can have different handlers, or a wildcard handler
//! can process all notifications.
//!
//! # Examples
//!
//! ```rust,no_run
//! use jrow_client::JrowClient;
//!
//! # async fn example(client: &JrowClient) {
//! // Register handler for specific topic
//! client.on_notification("events.user.login", |notif| async move {
//!     println!("User login event: {:?}", notif.params);
//! }).await;
//!
//! // Subscribe to receive notifications
//! client.subscribe("events.user.*", |value| async move {
//!     println!("Event data: {:?}", value);
//! }).await.unwrap();
//! # }
//! ```

use jrow_core::JsonRpcNotification;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Type for notification handler functions
pub type NotificationFn =
    Arc<dyn Fn(JsonRpcNotification) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Handler for incoming notifications
#[derive(Clone)]
pub struct NotificationHandler {
    handlers: Arc<Mutex<HashMap<String, NotificationFn>>>,
}

impl NotificationHandler {
    /// Create a new notification handler
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a handler for a notification method
    pub async fn register<F, Fut>(&self, method: impl Into<String>, handler: F)
    where
        F: Fn(JsonRpcNotification) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler: NotificationFn = Arc::new(move |notif| Box::pin(handler(notif)));
        self.handlers.lock().await.insert(method.into(), handler);
    }

    /// Handle an incoming notification
    pub async fn handle(&self, notification: JsonRpcNotification) {
        let method = notification.method.clone();
        let handlers = self.handlers.lock().await;

        if let Some(handler) = handlers.get(&method) {
            let handler = Arc::clone(handler);
            drop(handlers); // Release the lock before calling the handler

            handler(notification).await;
        } else {
            eprintln!("No handler registered for notification: {}", method);
        }
    }

    /// Check if a handler is registered for a method
    pub async fn has_handler(&self, method: &str) -> bool {
        self.handlers.lock().await.contains_key(method)
    }

    /// Remove a handler for a method
    pub async fn unregister(&self, method: &str) -> bool {
        self.handlers.lock().await.remove(method).is_some()
    }

    /// Get all registered notification methods
    pub async fn methods(&self) -> Vec<String> {
        self.handlers.lock().await.keys().cloned().collect()
    }
}

impl Default for NotificationHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn test_notification_handler() {
        let handler = NotificationHandler::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        handler
            .register("test", move |_notif| {
                let called = Arc::clone(&called_clone);
                async move {
                    called.store(true, Ordering::SeqCst);
                }
            })
            .await;

        assert!(handler.has_handler("test").await);

        let notification = JsonRpcNotification::new("test", None);
        handler.handle(notification).await;

        // Give the handler a moment to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_unregister() {
        let handler = NotificationHandler::new();

        handler.register("test", |_| async {}).await;
        assert!(handler.has_handler("test").await);

        handler.unregister("test").await;
        assert!(!handler.has_handler("test").await);
    }

    #[tokio::test]
    async fn test_multiple_handlers() {
        let handler = NotificationHandler::new();

        handler.register("method1", |_| async {}).await;
        handler.register("method2", |_| async {}).await;

        let methods = handler.methods().await;
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&"method1".to_string()));
        assert!(methods.contains(&"method2".to_string()));
    }
}


