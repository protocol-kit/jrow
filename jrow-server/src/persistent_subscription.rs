//! Persistent subscription management
//!
//! This module manages the lifecycle of persistent (durable) subscriptions.
//! Unlike regular subscriptions, persistent subscriptions:
//! - Survive client disconnects
//! - Track which messages have been acknowledged
//! - Replay unacknowledged messages on reconnect
//!
//! # Subscription Lifecycle
//!
//! 1. **Register**: Client creates subscription with unique ID
//! 2. **Deliver**: Messages matching the pattern are delivered
//! 3. **Acknowledge**: Client acks each message after processing
//! 4. **Disconnect**: Subscription state persisted to storage
//! 5. **Reconnect**: Resume from last acknowledged position
//!
//! # Exclusivity
//!
//! Subscriptions are exclusive - only one connection can be active for
//! a subscription ID at a time. This prevents duplicate delivery.
//!
//! # Inactivity Timeout
//!
//! Optionally, subscriptions can expire after a period of inactivity.
//! This prevents abandoned subscriptions from accumulating.
//!
//! # Examples
//!
//! ```rust,no_run
//! use jrow_server::{PersistentSubscriptionManager, PersistentStorage};
//! use std::sync::Arc;
//!
//! # async fn example() -> jrow_core::Result<()> {
//! let storage = Arc::new(PersistentStorage::new("./data")?);
//! let manager = PersistentSubscriptionManager::new(storage, None);
//!
//! // Register a subscription
//! let state = manager.register_subscription(
//!     "user-123-orders".to_string(),
//!     "orders.*".to_string(),
//!     1
//! ).await?;
//!
//! // After processing a message, acknowledge it
//! manager.acknowledge_message("user-123-orders", 42, 1).await?;
//! # Ok(())
//! # }
//! ```

use crate::persistent_storage::{PersistentStorage, SubscriptionState};
use crate::NatsPattern;
use jrow_core::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Information about an active persistent subscription
#[derive(Debug, Clone)]
struct SubscriptionInfo {
    subscription_id: String,
    pattern: NatsPattern,
    connection_id: u64,
}

/// Manages persistent subscriptions and their active connections
#[derive(Clone)]
pub struct PersistentSubscriptionManager {
    /// Map of subscription_id -> SubscriptionInfo for active subscriptions
    active_subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
    /// Map of connection_id -> set of subscription_ids
    connection_subscriptions: Arc<RwLock<HashMap<u64, Vec<String>>>>,
    /// Storage backend
    storage: Arc<PersistentStorage>,
    /// Optional timeout for inactive subscriptions (None = no timeout)
    inactivity_timeout: Option<Duration>,
}

impl PersistentSubscriptionManager {
    /// Create a new persistent subscription manager
    pub fn new(storage: Arc<PersistentStorage>, inactivity_timeout: Option<Duration>) -> Self {
        Self {
            active_subscriptions: Arc::new(RwLock::new(HashMap::new())),
            connection_subscriptions: Arc::new(RwLock::new(HashMap::new())),
            storage,
            inactivity_timeout,
        }
    }

    /// Register a persistent subscription (enforce exclusivity)
    /// The topic parameter can be an exact topic or a NATS-style pattern
    pub async fn register_subscription(
        &self,
        subscription_id: String,
        topic: String,
        connection_id: u64,
    ) -> Result<SubscriptionState> {
        // Parse the pattern
        let pattern = NatsPattern::new(&topic)
            .map_err(|e| Error::InvalidParams(format!("Invalid topic pattern: {}", e)))?;
        
        let mut active = self.active_subscriptions.write().await;
        
        // Check if subscription is already active on another connection
        if let Some(existing_info) = active.get(&subscription_id) {
            if existing_info.connection_id != connection_id {
                return Err(Error::InvalidRequest(format!(
                    "Subscription '{}' is already active on another connection",
                    subscription_id
                )));
            }
        }
        
        // Get or create subscription state in storage
        let mut state = self.storage.create_subscription(&subscription_id, &topic).await?;
        
        // Update state with pattern info
        state.topic_pattern = Some(topic.clone());
        
        // Mark as active with pattern info
        active.insert(subscription_id.clone(), SubscriptionInfo {
            subscription_id: subscription_id.clone(),
            pattern,
            connection_id,
        });
        
        // Track for this connection
        let mut conn_subs = self.connection_subscriptions.write().await;
        conn_subs
            .entry(connection_id)
            .or_insert_with(Vec::new)
            .push(subscription_id);
        
        Ok(state)
    }

    /// Acknowledge a message (update last ack position)
    pub async fn acknowledge_message(
        &self,
        subscription_id: &str,
        sequence_id: u64,
        connection_id: u64,
    ) -> Result<()> {
        // Verify subscription is active on this connection
        let active = self.active_subscriptions.read().await;
        match active.get(subscription_id) {
            Some(info) if info.connection_id == connection_id => {
                // Update storage
                self.storage
                    .update_subscription_position(subscription_id, sequence_id)
                    .await
            }
            Some(_) => Err(Error::InvalidRequest(format!(
                "Subscription '{}' is not active on this connection",
                subscription_id
            ))),
            None => Err(Error::InvalidRequest(format!(
                "Subscription '{}' is not active",
                subscription_id
            ))),
        }
    }

    /// Unsubscribe (remove from active, but keep state in storage for resume)
    pub async fn unsubscribe(&self, subscription_id: &str, connection_id: u64) -> Result<bool> {
        let mut active = self.active_subscriptions.write().await;
        
        // Check if subscription exists and belongs to this connection
        match active.get(subscription_id) {
            Some(info) if info.connection_id == connection_id => {
                active.remove(subscription_id);
                
                // Remove from connection tracking
                let mut conn_subs = self.connection_subscriptions.write().await;
                if let Some(subs) = conn_subs.get_mut(&connection_id) {
                    subs.retain(|s| s != subscription_id);
                    if subs.is_empty() {
                        conn_subs.remove(&connection_id);
                    }
                }
                
                Ok(true)
            }
            Some(_) => Err(Error::InvalidRequest(format!(
                "Subscription '{}' belongs to another connection",
                subscription_id
            ))),
            None => Ok(false),
        }
    }

    /// Remove all subscriptions for a connection (cleanup on disconnect)
    pub async fn remove_connection(&self, connection_id: u64) {
        let mut conn_subs = self.connection_subscriptions.write().await;
        
        if let Some(subscription_ids) = conn_subs.remove(&connection_id) {
            let mut active = self.active_subscriptions.write().await;
            for sub_id in subscription_ids {
                active.remove(&sub_id);
            }
        }
    }

    /// Check if a subscription is active
    pub async fn is_active(&self, subscription_id: &str) -> bool {
        let active = self.active_subscriptions.read().await;
        active.contains_key(subscription_id)
    }

    /// Get the connection ID for an active subscription
    pub async fn get_connection_id(&self, subscription_id: &str) -> Option<u64> {
        let active = self.active_subscriptions.read().await;
        active.get(subscription_id).map(|info| info.connection_id)
    }

    /// Get all subscriptions whose patterns match a given topic
    /// Returns a vector of (subscription_id, connection_id) pairs
    pub async fn get_matching_subscriptions(&self, topic: &str) -> Vec<(String, u64)> {
        let active = self.active_subscriptions.read().await;
        active
            .values()
            .filter(|info| info.pattern.matches(topic))
            .map(|info| (info.subscription_id.clone(), info.connection_id))
            .collect()
    }

    /// Get all subscriptions for a connection
    pub async fn get_connection_subscriptions(&self, connection_id: u64) -> Vec<String> {
        let conn_subs = self.connection_subscriptions.read().await;
        conn_subs
            .get(&connection_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Clean up inactive subscriptions based on timeout
    pub async fn cleanup_inactive_subscriptions(&self) -> Result<Vec<String>> {
        let timeout = match self.inactivity_timeout {
            Some(t) => t,
            None => return Ok(Vec::new()), // No timeout configured
        };
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let timeout_secs = timeout.as_secs();
        let mut deleted = Vec::new();
        
        // Get all subscriptions from storage
        let all_subs = self.storage.get_all_subscriptions().await?;
        let active = self.active_subscriptions.read().await;
        
        for sub in all_subs {
            // Skip active subscriptions
            if active.contains_key(&sub.subscription_id) {
                continue;
            }
            
            // Check if inactive for too long
            let inactive_duration = now.saturating_sub(sub.last_activity);
            if inactive_duration > timeout_secs {
                tracing::info!(
                    subscription_id = %sub.subscription_id,
                    inactive_secs = inactive_duration,
                    "Cleaning up inactive subscription"
                );
                
                if self.storage.delete_subscription(&sub.subscription_id).await? {
                    deleted.push(sub.subscription_id);
                }
            }
        }
        
        Ok(deleted)
    }

    /// Get count of active subscriptions
    pub async fn active_count(&self) -> usize {
        let active = self.active_subscriptions.read().await;
        active.len()
    }

    /// Get storage reference
    pub fn storage(&self) -> &Arc<PersistentStorage> {
        &self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RetentionPolicy;

    async fn create_test_manager() -> PersistentSubscriptionManager {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(PersistentStorage::new(temp_dir.path()).unwrap());
        PersistentSubscriptionManager::new(storage, None)
    }

    #[tokio::test]
    async fn test_register_subscription() {
        let manager = create_test_manager().await;
        
        let state = manager
            .register_subscription("sub1".to_string(), "topic1".to_string(), 1)
            .await
            .unwrap();
        
        assert_eq!(state.subscription_id, "sub1");
        assert_eq!(state.topic, "topic1");
        assert!(manager.is_active("sub1").await);
        assert_eq!(manager.get_connection_id("sub1").await, Some(1));
    }

    #[tokio::test]
    async fn test_exclusive_subscription() {
        let manager = create_test_manager().await;
        
        // Register on connection 1
        manager
            .register_subscription("sub1".to_string(), "topic1".to_string(), 1)
            .await
            .unwrap();
        
        // Try to register same subscription on connection 2
        let result = manager
            .register_subscription("sub1".to_string(), "topic1".to_string(), 2)
            .await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_acknowledge_message() {
        let manager = create_test_manager().await;
        
        manager
            .register_subscription("sub1".to_string(), "topic1".to_string(), 1)
            .await
            .unwrap();
        
        manager.acknowledge_message("sub1", 10, 1).await.unwrap();
        
        let state = manager.storage().get_subscription_state("sub1").await.unwrap().unwrap();
        assert_eq!(state.last_ack_seq, 10);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let manager = create_test_manager().await;
        
        manager
            .register_subscription("sub1".to_string(), "topic1".to_string(), 1)
            .await
            .unwrap();
        
        let removed = manager.unsubscribe("sub1", 1).await.unwrap();
        assert!(removed);
        assert!(!manager.is_active("sub1").await);
        
        // State should still exist in storage
        let state = manager.storage().get_subscription_state("sub1").await.unwrap();
        assert!(state.is_some());
    }

    #[tokio::test]
    async fn test_remove_connection() {
        let manager = create_test_manager().await;
        
        manager
            .register_subscription("sub1".to_string(), "topic1".to_string(), 1)
            .await
            .unwrap();
        manager
            .register_subscription("sub2".to_string(), "topic2".to_string(), 1)
            .await
            .unwrap();
        
        manager.remove_connection(1).await;
        
        assert!(!manager.is_active("sub1").await);
        assert!(!manager.is_active("sub2").await);
        assert_eq!(manager.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_cleanup_inactive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(PersistentStorage::new(temp_dir.path()).unwrap());
        
        // Create manager with 1 second timeout
        let manager = PersistentSubscriptionManager::new(
            storage.clone(),
            Some(Duration::from_secs(1)),
        );
        
        // Create and activate subscription
        manager
            .register_subscription("sub1".to_string(), "topic1".to_string(), 1)
            .await
            .unwrap();
        
        // Unsubscribe (makes it inactive)
        manager.unsubscribe("sub1", 1).await.unwrap();
        
        // Wait for timeout
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Cleanup should remove it
        let deleted = manager.cleanup_inactive_subscriptions().await.unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0], "sub1");
    }

    #[tokio::test]
    async fn test_pattern_subscription_exact() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(PersistentStorage::new(temp_dir.path()).unwrap());
        let manager = PersistentSubscriptionManager::new(storage, None);

        // Subscribe with exact topic
        let state = manager
            .register_subscription("sub1".to_string(), "orders.new".to_string(), 1)
            .await
            .unwrap();

        assert_eq!(state.topic, "orders.new");
        assert_eq!(state.topic_pattern, Some("orders.new".to_string()));

        // Should match exact topic
        let matches = manager.get_matching_subscriptions("orders.new").await;
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, "sub1");

        // Should not match different topic
        let matches = manager.get_matching_subscriptions("orders.shipped").await;
        assert_eq!(matches.len(), 0);
    }

    #[tokio::test]
    async fn test_pattern_subscription_single_wildcard() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(PersistentStorage::new(temp_dir.path()).unwrap());
        let manager = PersistentSubscriptionManager::new(storage, None);

        // Subscribe with single wildcard pattern
        let state = manager
            .register_subscription("sub1".to_string(), "orders.*".to_string(), 1)
            .await
            .unwrap();

        assert_eq!(state.topic, "orders.*");
        assert_eq!(state.topic_pattern, Some("orders.*".to_string()));

        // Should match topics with one token after orders
        let matches = manager.get_matching_subscriptions("orders.new").await;
        assert_eq!(matches.len(), 1);

        let matches = manager.get_matching_subscriptions("orders.shipped").await;
        assert_eq!(matches.len(), 1);

        let matches = manager.get_matching_subscriptions("orders.cancelled").await;
        assert_eq!(matches.len(), 1);

        // Should not match without suffix
        let matches = manager.get_matching_subscriptions("orders").await;
        assert_eq!(matches.len(), 0);

        // Should not match with multiple tokens
        let matches = manager.get_matching_subscriptions("orders.new.fast").await;
        assert_eq!(matches.len(), 0);

        // Should not match different prefix
        let matches = manager.get_matching_subscriptions("events.new").await;
        assert_eq!(matches.len(), 0);
    }

    #[tokio::test]
    async fn test_pattern_subscription_multi_wildcard() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(PersistentStorage::new(temp_dir.path()).unwrap());
        let manager = PersistentSubscriptionManager::new(storage, None);

        // Subscribe with multi wildcard pattern
        let state = manager
            .register_subscription("sub1".to_string(), "events.>".to_string(), 1)
            .await
            .unwrap();

        assert_eq!(state.topic, "events.>");
        assert_eq!(state.topic_pattern, Some("events.>".to_string()));

        // Should match with one token
        let matches = manager.get_matching_subscriptions("events.user").await;
        assert_eq!(matches.len(), 1);

        // Should match with multiple tokens
        let matches = manager.get_matching_subscriptions("events.user.login").await;
        assert_eq!(matches.len(), 1);

        let matches = manager.get_matching_subscriptions("events.user.login.success").await;
        assert_eq!(matches.len(), 1);

        // Should not match without suffix
        let matches = manager.get_matching_subscriptions("events").await;
        assert_eq!(matches.len(), 0);

        // Should not match different prefix
        let matches = manager.get_matching_subscriptions("orders.user").await;
        assert_eq!(matches.len(), 0);
    }

    #[tokio::test]
    async fn test_pattern_subscription_multiple_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(PersistentStorage::new(temp_dir.path()).unwrap());
        let manager = PersistentSubscriptionManager::new(storage, None);

        // Register multiple subscriptions with different patterns
        manager
            .register_subscription("exact".to_string(), "orders.new".to_string(), 1)
            .await
            .unwrap();

        manager
            .register_subscription("single".to_string(), "orders.*".to_string(), 2)
            .await
            .unwrap();

        manager
            .register_subscription("multi".to_string(), "orders.>".to_string(), 3)
            .await
            .unwrap();

        // Test orders.new - should match all three
        let matches = manager.get_matching_subscriptions("orders.new").await;
        assert_eq!(matches.len(), 3);
        let sub_ids: Vec<String> = matches.iter().map(|(id, _)| id.clone()).collect();
        assert!(sub_ids.contains(&"exact".to_string()));
        assert!(sub_ids.contains(&"single".to_string()));
        assert!(sub_ids.contains(&"multi".to_string()));

        // Test orders.shipped - should match single and multi only
        let matches = manager.get_matching_subscriptions("orders.shipped").await;
        assert_eq!(matches.len(), 2);
        let sub_ids: Vec<String> = matches.iter().map(|(id, _)| id.clone()).collect();
        assert!(sub_ids.contains(&"single".to_string()));
        assert!(sub_ids.contains(&"multi".to_string()));

        // Test orders.new.fast - should match multi only
        let matches = manager.get_matching_subscriptions("orders.new.fast").await;
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, "multi");

        // Test events.new - should match none
        let matches = manager.get_matching_subscriptions("events.new").await;
        assert_eq!(matches.len(), 0);
    }

    #[tokio::test]
    async fn test_pattern_subscription_invalid_pattern() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(PersistentStorage::new(temp_dir.path()).unwrap());
        let manager = PersistentSubscriptionManager::new(storage, None);

        // Invalid: > not at end
        let result = manager
            .register_subscription("sub1".to_string(), "orders.>.new".to_string(), 1)
            .await;
        assert!(result.is_err());

        // Invalid: mixed wildcards
        let result = manager
            .register_subscription("sub2".to_string(), "orders.*.>".to_string(), 1)
            .await;
        assert!(result.is_err());

        // Invalid: empty token
        let result = manager
            .register_subscription("sub3".to_string(), "orders..new".to_string(), 1)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pattern_subscription_resume_with_pattern() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(PersistentStorage::new(temp_dir.path()).unwrap());
        let manager = PersistentSubscriptionManager::new(storage.clone(), None);

        // Subscribe with pattern
        manager
            .register_subscription("sub1".to_string(), "orders.*".to_string(), 1)
            .await
            .unwrap();

        // Store multiple messages in matching topics
        // orders.new: seq 1, 2
        storage.store_message("orders.new", serde_json::json!({"order": 1})).await.unwrap();
        storage.store_message("orders.new", serde_json::json!({"order": 2})).await.unwrap();
        
        // orders.shipped: seq 1
        storage.store_message("orders.shipped", serde_json::json!({"order": 3})).await.unwrap();

        // Get all messages matching pattern (from beginning)
        let pattern = crate::NatsPattern::new("orders.*").unwrap();
        let messages = storage.get_messages_matching_pattern(&pattern, 0).await.unwrap();

        // Should get all 3 messages, sorted by (topic, sequence_id)
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].topic, "orders.new");
        assert_eq!(messages[0].sequence_id, 1);
        assert_eq!(messages[1].topic, "orders.new");
        assert_eq!(messages[1].sequence_id, 2);
        assert_eq!(messages[2].topic, "orders.shipped");
        assert_eq!(messages[2].sequence_id, 1);

        // Acknowledge first message (orders.new seq 1)
        manager.acknowledge_message("sub1", 1, 1).await.unwrap();

        // Get messages since last ack (seq > 1)
        let messages = storage.get_messages_matching_pattern(&pattern, 1).await.unwrap();

        // Should get messages with seq > 1: orders.new/2 and orders.shipped/1
        // But actually since we're checking seq > 1, we only get orders.new/2
        // orders.shipped/1 has seq=1 which is not > 1
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].topic, "orders.new");
        assert_eq!(messages[0].sequence_id, 2);
    }
}
