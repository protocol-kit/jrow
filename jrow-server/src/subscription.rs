//! Subscription management for pub/sub functionality
//!
//! This module implements the server-side subscription tracking for pub/sub.
//! It maintains bidirectional mappings between connections and topics to enable:
//! - Fast lookup of subscribers for a topic (for publishing)
//! - Fast cleanup of all subscriptions when a connection closes
//!
//! # Data Structures
//!
//! The manager uses two HashMaps for bidirectional lookups:
//! - `topic -> Set<connection_id>`: Find subscribers for a topic
//! - `connection_id -> Set<topic>`: Find topics for a connection
//!
//! Both mappings are kept in sync to ensure consistency.
//!
//! # Thread Safety
//!
//! The manager is `Clone` and thread-safe, using `Arc<Mutex<...>>` for
//! shared mutable state. This allows multiple connection tasks to share
//! the same subscription manager.
//!
//! # Examples
//!
//! ```rust
//! use jrow_server::SubscriptionManager;
//!
//! # async fn example() {
//! let manager = SubscriptionManager::new();
//!
//! // Connection 1 subscribes to "events"
//! manager.subscribe(1, "events").await;
//!
//! // Connection 2 subscribes to "events" and "logs"
//! manager.subscribe(2, "events").await;
//! manager.subscribe(2, "logs").await;
//!
//! // Get all subscribers for "events"
//! let subscribers = manager.get_subscribers("events").await;
//! assert_eq!(subscribers.len(), 2); // connections 1 and 2
//!
//! // When connection 1 disconnects, clean up
//! manager.remove_connection(1).await;
//! # }
//! ```

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Manages topic subscriptions for connections
///
/// Maintains bidirectional mappings for efficient lookup in both directions:
/// - Given a topic, find all subscribed connections (for publishing)
/// - Given a connection, find all subscribed topics (for cleanup)
///
/// # Implementation Notes
///
/// Uses `HashSet` for subscriber lists to ensure uniqueness and O(1)
/// insertion/removal. The dual-map design trades memory for speed.
#[derive(Clone)]
pub struct SubscriptionManager {
    /// Map of topic -> set of connection IDs subscribed to that topic
    /// Used when publishing: quickly find who to send to
    topic_subscribers: Arc<Mutex<HashMap<String, HashSet<u64>>>>,
    
    /// Map of connection ID -> set of topics that connection is subscribed to
    /// Used when disconnecting: quickly find what to clean up
    connection_topics: Arc<Mutex<HashMap<u64, HashSet<String>>>>,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self {
            topic_subscribers: Arc::new(Mutex::new(HashMap::new())),
            connection_topics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Subscribe a connection to a topic
    pub async fn subscribe(&self, connection_id: u64, topic: impl Into<String>) -> bool {
        let topic = topic.into();

        // Add to topic_subscribers
        let mut topic_subs = self.topic_subscribers.lock().await;
        let subscribers = topic_subs.entry(topic.clone()).or_insert_with(HashSet::new);
        let is_new = subscribers.insert(connection_id);
        drop(topic_subs);

        // Add to connection_topics
        let mut conn_topics = self.connection_topics.lock().await;
        let topics = conn_topics
            .entry(connection_id)
            .or_insert_with(HashSet::new);
        topics.insert(topic);

        is_new
    }

    /// Unsubscribe a connection from a topic
    pub async fn unsubscribe(&self, connection_id: u64, topic: &str) -> bool {
        let mut removed = false;

        // Remove from topic_subscribers
        let mut topic_subs = self.topic_subscribers.lock().await;
        if let Some(subscribers) = topic_subs.get_mut(topic) {
            removed = subscribers.remove(&connection_id);
            if subscribers.is_empty() {
                topic_subs.remove(topic);
            }
        }
        drop(topic_subs);

        // Remove from connection_topics
        let mut conn_topics = self.connection_topics.lock().await;
        if let Some(topics) = conn_topics.get_mut(&connection_id) {
            topics.remove(topic);
            if topics.is_empty() {
                conn_topics.remove(&connection_id);
            }
        }

        removed
    }

    /// Get all connection IDs subscribed to a topic
    pub async fn get_subscribers(&self, topic: &str) -> Vec<u64> {
        let topic_subs = self.topic_subscribers.lock().await;
        topic_subs
            .get(topic)
            .map(|subs| subs.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get all topics a connection is subscribed to
    pub async fn get_topics(&self, connection_id: u64) -> Vec<String> {
        let conn_topics = self.connection_topics.lock().await;
        conn_topics
            .get(&connection_id)
            .map(|topics| topics.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Remove all subscriptions for a connection (cleanup on disconnect)
    pub async fn remove_connection(&self, connection_id: u64) {
        // Get all topics this connection is subscribed to
        let topics = {
            let mut conn_topics = self.connection_topics.lock().await;
            conn_topics.remove(&connection_id)
        };

        if let Some(topics) = topics {
            // Remove connection from all those topics
            let mut topic_subs = self.topic_subscribers.lock().await;
            for topic in topics {
                if let Some(subscribers) = topic_subs.get_mut(&topic) {
                    subscribers.remove(&connection_id);
                    if subscribers.is_empty() {
                        topic_subs.remove(&topic);
                    }
                }
            }
        }
    }

    /// Get the total number of active subscriptions
    pub async fn subscription_count(&self) -> usize {
        let conn_topics = self.connection_topics.lock().await;
        conn_topics.values().map(|topics| topics.len()).sum()
    }

    /// Get the number of unique topics with subscribers
    pub async fn topic_count(&self) -> usize {
        let topic_subs = self.topic_subscribers.lock().await;
        topic_subs.len()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscribe() {
        let manager = SubscriptionManager::new();

        let is_new = manager.subscribe(1, "topic1").await;
        assert!(is_new);

        let subscribers = manager.get_subscribers("topic1").await;
        assert_eq!(subscribers, vec![1]);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let manager = SubscriptionManager::new();

        manager.subscribe(1, "topic1").await;
        manager.subscribe(2, "topic1").await;
        manager.subscribe(3, "topic1").await;

        let mut subscribers = manager.get_subscribers("topic1").await;
        subscribers.sort();
        assert_eq!(subscribers, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let manager = SubscriptionManager::new();

        manager.subscribe(1, "topic1").await;
        manager.subscribe(2, "topic1").await;

        let removed = manager.unsubscribe(1, "topic1").await;
        assert!(removed);

        let subscribers = manager.get_subscribers("topic1").await;
        assert_eq!(subscribers, vec![2]);
    }

    #[tokio::test]
    async fn test_get_topics() {
        let manager = SubscriptionManager::new();

        manager.subscribe(1, "topic1").await;
        manager.subscribe(1, "topic2").await;
        manager.subscribe(1, "topic3").await;

        let mut topics = manager.get_topics(1).await;
        topics.sort();
        assert_eq!(topics, vec!["topic1", "topic2", "topic3"]);
    }

    #[tokio::test]
    async fn test_remove_connection() {
        let manager = SubscriptionManager::new();

        manager.subscribe(1, "topic1").await;
        manager.subscribe(1, "topic2").await;
        manager.subscribe(2, "topic1").await;

        manager.remove_connection(1).await;

        let subscribers = manager.get_subscribers("topic1").await;
        assert_eq!(subscribers, vec![2]);

        let subscribers = manager.get_subscribers("topic2").await;
        assert!(subscribers.is_empty());
    }

    #[tokio::test]
    async fn test_counts() {
        let manager = SubscriptionManager::new();

        manager.subscribe(1, "topic1").await;
        manager.subscribe(1, "topic2").await;
        manager.subscribe(2, "topic1").await;

        assert_eq!(manager.subscription_count().await, 3);
        assert_eq!(manager.topic_count().await, 2);
    }
}


