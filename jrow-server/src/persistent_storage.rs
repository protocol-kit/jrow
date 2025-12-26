//! Persistent storage implementation using sled database
//!
//! This module provides durable message storage for persistent subscriptions.
//! It uses sled (an embedded database) to store messages on disk, enabling:
//! - Message persistence across server restarts
//! - Message replay for subscribers that reconnect
//! - Retention policies for automatic cleanup
//!
//! # Data Model
//!
//! The storage maintains three trees (tables):
//! - **messages**: Individual messages with sequence IDs
//! - **subscriptions**: Subscription state (last acknowledged sequence)
//! - **metadata**: Topic metadata (sequence counters, retention policies)
//!
//! # Sequence IDs
//!
//! Each message gets a monotonically increasing sequence ID per topic.
//! Sequence IDs are used to track delivery progress and enable replay
//! from a specific point.
//!
//! # Retention Policies
//!
//! Policies control how long messages are kept:
//! - **Time-based**: Keep messages for N seconds
//! - **Count-based**: Keep last N messages
//! - **Size-based**: Keep messages up to N bytes
//! - **Unlimited**: Keep all messages (not recommended for production)
//!
//! A background task periodically enforces retention policies.
//!
//! # Examples
//!
//! ```rust,no_run
//! use jrow_server::{PersistentStorage, RetentionPolicy};
//! use std::time::Duration;
//!
//! # async fn example() -> jrow_core::Result<()> {
//! let storage = PersistentStorage::new("./data/jrow.db")?;
//!
//! // Register a topic with retention
//! storage.register_topic(
//!     "events",
//!     RetentionPolicy::by_age(Duration::from_secs(86400)) // 24 hours
//! ).await?;
//!
//! // Store a message
//! let seq_id = storage.store_message(
//!     "events.user.login",
//!     serde_json::json!({"user_id": 123})
//! ).await?;
//! # Ok(())
//! # }
//! ```

use crate::retention::RetentionPolicy;
use jrow_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// A persistent message stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentMessage {
    pub sequence_id: u64,
    pub topic: String,
    pub data: String, // JSON string for bincode compatibility
    pub timestamp: u64,
    pub size_bytes: usize,
}

impl PersistentMessage {
    /// Get the data as a serde_json::Value
    pub fn data_as_value(&self) -> Result<serde_json::Value> {
        serde_json::from_str(&self.data)
            .map_err(|e| Error::Internal(format!("Failed to parse message data: {}", e)))
    }
}

/// Subscription state for persistent tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionState {
    pub subscription_id: String,
    pub topic: String,  // Kept for backward compatibility
    pub topic_pattern: Option<String>,  // Pattern string (exact topics have None)
    pub last_ack_seq: u64,
    pub last_ack_topic: Option<String>,  // Track which topic was last acked (for patterns)
    pub created_at: u64,
    pub last_activity: u64,
}

/// Topic metadata including retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicMetadata {
    pub topic: String,
    pub max_sequence: u64,
    pub retention_policy: RetentionPolicy,
    pub message_count: usize,
    pub total_bytes: usize,
}

/// Persistent storage backend using sled
pub struct PersistentStorage {
    #[allow(dead_code)]
    db: sled::Db,
    messages_tree: sled::Tree,
    subscriptions_tree: sled::Tree,
    metadata_tree: sled::Tree,
    topic_metadata_cache: Arc<RwLock<HashMap<String, TopicMetadata>>>,
}

impl PersistentStorage {
    /// Create a new persistent storage instance
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::open(db_path).map_err(|e| Error::Internal(format!("Failed to open sled database: {}", e)))?;
        
        let messages_tree = db
            .open_tree("messages")
            .map_err(|e| Error::Internal(format!("Failed to open messages tree: {}", e)))?;
        
        let subscriptions_tree = db
            .open_tree("subscriptions")
            .map_err(|e| Error::Internal(format!("Failed to open subscriptions tree: {}", e)))?;
        
        let metadata_tree = db
            .open_tree("metadata")
            .map_err(|e| Error::Internal(format!("Failed to open metadata tree: {}", e)))?;
        
        // Load metadata into cache synchronously
        let metadata_tree_clone = metadata_tree.clone();
        let mut initial_cache = HashMap::new();
        
        for item in metadata_tree_clone.iter() {
            if let Ok((key, value)) = item {
                if let (Ok(topic), Ok(metadata)) = (
                    String::from_utf8(key.to_vec()),
                    bincode::deserialize::<TopicMetadata>(&value)
                ) {
                    initial_cache.insert(topic, metadata);
                }
            }
        }
        
        Ok(Self {
            db,
            messages_tree,
            subscriptions_tree,
            metadata_tree,
            topic_metadata_cache: Arc::new(RwLock::new(initial_cache)),
        })
    }

    /// Load all topic metadata into the cache (synchronous)
    fn load_metadata_cache_sync(&self) -> Result<HashMap<String, TopicMetadata>> {
        let mut cache = HashMap::new();
        
        for item in self.metadata_tree.iter() {
            let (key, value) = item.map_err(|e| Error::Internal(format!("Failed to read metadata: {}", e)))?;
            let topic = String::from_utf8(key.to_vec())
                .map_err(|e| Error::Internal(format!("Invalid topic key: {}", e)))?;
            let metadata: TopicMetadata = bincode::deserialize(&value)
                .map_err(|e| Error::Internal(format!("Failed to deserialize metadata: {}", e)))?;
            cache.insert(topic, metadata);
        }
        
        Ok(cache)
    }

    /// Register a topic with a retention policy
    pub async fn register_topic(&self, topic: impl Into<String>, retention_policy: RetentionPolicy) -> Result<()> {
        let topic = topic.into();
        
        // Check if topic already has metadata (preserve sequence numbers and counts)
        let existing_metadata = {
            let cache = self.topic_metadata_cache.read().await;
            cache.get(&topic).cloned()
        };
        
        let metadata = if let Some(mut existing) = existing_metadata {
            // Update only the retention policy, preserve sequence and counts
            existing.retention_policy = retention_policy;
            existing
        } else {
            // New topic, create fresh metadata
            TopicMetadata {
                topic: topic.clone(),
                max_sequence: 0,
                retention_policy,
                message_count: 0,
                total_bytes: 0,
            }
        };
        
        // Store in sled
        let key = topic.as_bytes();
        let value = bincode::serialize(&metadata)
            .map_err(|e| Error::Internal(format!("Failed to serialize metadata: {}", e)))?;
        
        self.metadata_tree
            .insert(key, value)
            .map_err(|e| Error::Internal(format!("Failed to store metadata: {}", e)))?;
        
        self.metadata_tree
            .flush_async()
            .await
            .map_err(|e| Error::Internal(format!("Failed to flush metadata: {}", e)))?;
        
        // Update cache
        self.topic_metadata_cache.write().await.insert(topic, metadata);
        
        Ok(())
    }

    /// Store a message and return its sequence ID
    pub async fn store_message(&self, topic: impl Into<String>, data: serde_json::Value) -> Result<u64> {
        let topic = topic.into();
        
        // Get topic metadata from cache, or load from database if not cached
        let mut metadata = {
            let cache = self.topic_metadata_cache.read().await;
            if let Some(meta) = cache.get(&topic).cloned() {
                meta
            } else {
                drop(cache);
                // Not in cache, check database
                if let Some(db_value) = self.metadata_tree.get(topic.as_bytes())
                    .map_err(|e| Error::Internal(format!("Failed to read metadata: {}", e)))? 
                {
                    // Found in database, deserialize and add to cache
                    let meta: TopicMetadata = bincode::deserialize(&db_value)
                        .map_err(|e| Error::Internal(format!("Failed to deserialize metadata: {}", e)))?;
                    self.topic_metadata_cache.write().await.insert(topic.clone(), meta.clone());
                    meta
                } else {
                    // Not in database either, create new
                    TopicMetadata {
                        topic: topic.clone(),
                        max_sequence: 0,
                        retention_policy: RetentionPolicy::unlimited(),
                        message_count: 0,
                        total_bytes: 0,
                    }
                }
            }
        };
        
        // Increment sequence
        metadata.max_sequence += 1;
        let sequence_id = metadata.max_sequence;
        
        // Create message
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Store data as JSON string for bincode compatibility
        let data_str = data.to_string();
        let size_bytes = data_str.len();
        
        let message = PersistentMessage {
            sequence_id,
            topic: topic.clone(),
            data: data_str,
            timestamp,
            size_bytes,
        };
        
        // Serialize and store
        let key = format!("{}:{:020}", topic, sequence_id);
        let value = bincode::serialize(&message)
            .map_err(|e| Error::Internal(format!("Failed to serialize message: {}", e)))?;
        
        self.messages_tree
            .insert(key.as_bytes(), value)
            .map_err(|e| Error::Internal(format!("Failed to store message: {}", e)))?;
        
        // Update metadata
        metadata.message_count += 1;
        metadata.total_bytes += size_bytes;
        
        let metadata_key = topic.as_bytes();
        let metadata_value = bincode::serialize(&metadata)
            .map_err(|e| Error::Internal(format!("Failed to serialize metadata: {}", e)))?;
        
        self.metadata_tree
            .insert(metadata_key, metadata_value)
            .map_err(|e| Error::Internal(format!("Failed to update metadata: {}", e)))?;
        
        // Flush to disk
        self.messages_tree
            .flush_async()
            .await
            .map_err(|e| Error::Internal(format!("Failed to flush messages: {}", e)))?;
        
        self.metadata_tree
            .flush_async()
            .await
            .map_err(|e| Error::Internal(format!("Failed to flush metadata: {}", e)))?;
        
        // Update cache
        self.topic_metadata_cache.write().await.insert(topic, metadata);
        
        Ok(sequence_id)
    }

    /// Retrieve messages since a given sequence ID
    pub async fn get_messages_since(&self, topic: &str, since_seq: u64) -> Result<Vec<PersistentMessage>> {
        let mut messages = Vec::new();
        
        let prefix = format!("{}:", topic);
        let start_key = format!("{}:{:020}", topic, since_seq + 1);
        
        for item in self.messages_tree.range(start_key.as_bytes()..) {
            let (key, value) = item.map_err(|e| Error::Internal(format!("Failed to read message: {}", e)))?;
            
            // Check if still in the same topic
            let key_str = String::from_utf8(key.to_vec())
                .map_err(|e| Error::Internal(format!("Invalid message key: {}", e)))?;
            
            if !key_str.starts_with(&prefix) {
                break;
            }
            
            let message: PersistentMessage = bincode::deserialize(&value)
                .map_err(|e| Error::Internal(format!("Failed to deserialize message: {}", e)))?;
            
            messages.push(message);
        }
        
        Ok(messages)
    }

    /// Get messages matching a pattern since a given sequence ID
    /// Returns messages from all topics matching the pattern, sorted by (topic, sequence_id)
    pub async fn get_messages_matching_pattern(
        &self,
        pattern: &crate::NatsPattern,
        since_seq: u64,
    ) -> Result<Vec<PersistentMessage>> {
        // If exact pattern, use the optimized single-topic lookup
        if !pattern.is_pattern() {
            return self.get_messages_since(pattern.as_str(), since_seq).await;
        }

        // Get all topics
        let all_topics = self.get_all_topics().await?;
        
        // Filter topics matching the pattern
        let matching_topics: Vec<String> = all_topics
            .into_iter()
            .filter(|topic| pattern.matches(topic))
            .collect();

        // Get messages from each matching topic
        let mut all_messages = Vec::new();
        for topic in matching_topics {
            let messages = self.get_messages_since(&topic, since_seq).await?;
            all_messages.extend(messages);
        }

        // Sort by (topic, sequence_id) for deterministic order
        all_messages.sort_by(|a, b| {
            a.topic.cmp(&b.topic).then(a.sequence_id.cmp(&b.sequence_id))
        });

        Ok(all_messages)
    }

    /// Update subscription position (last acknowledged sequence)
    pub async fn update_subscription_position(&self, subscription_id: &str, sequence_id: u64) -> Result<()> {
        // Get existing subscription state or create new
        let mut state = self.get_subscription_state(subscription_id).await?
            .unwrap_or_else(|| SubscriptionState {
                subscription_id: subscription_id.to_string(),
                topic: String::new(),
                topic_pattern: None,
                last_ack_seq: 0,
                last_ack_topic: None,
                created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                last_activity: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            });
        
        state.last_ack_seq = sequence_id;
        state.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let key = subscription_id.as_bytes();
        let value = bincode::serialize(&state)
            .map_err(|e| Error::Internal(format!("Failed to serialize subscription: {}", e)))?;
        
        self.subscriptions_tree
            .insert(key, value)
            .map_err(|e| Error::Internal(format!("Failed to update subscription: {}", e)))?;
        
        self.subscriptions_tree
            .flush_async()
            .await
            .map_err(|e| Error::Internal(format!("Failed to flush subscription: {}", e)))?;
        
        Ok(())
    }

    /// Get subscription state
    pub async fn get_subscription_state(&self, subscription_id: &str) -> Result<Option<SubscriptionState>> {
        let key = subscription_id.as_bytes();
        
        match self.subscriptions_tree.get(key)
            .map_err(|e| Error::Internal(format!("Failed to get subscription: {}", e)))? {
            Some(value) => {
                let state: SubscriptionState = bincode::deserialize(&value)
                    .map_err(|e| Error::Internal(format!("Failed to deserialize subscription: {}", e)))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    /// Create or update subscription state
    pub async fn create_subscription(&self, subscription_id: &str, topic: &str) -> Result<SubscriptionState> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Check if subscription already exists
        if let Some(mut existing) = self.get_subscription_state(subscription_id).await? {
            existing.last_activity = now;
            
            let key = subscription_id.as_bytes();
            let value = bincode::serialize(&existing)
                .map_err(|e| Error::Internal(format!("Failed to serialize subscription: {}", e)))?;
            
            self.subscriptions_tree
                .insert(key, value)
                .map_err(|e| Error::Internal(format!("Failed to update subscription: {}", e)))?;
            
            self.subscriptions_tree
                .flush_async()
                .await
                .map_err(|e| Error::Internal(format!("Failed to flush subscription: {}", e)))?;
            
            return Ok(existing);
        }
        
        // Create new subscription
        let state = SubscriptionState {
            subscription_id: subscription_id.to_string(),
            topic: topic.to_string(),
            topic_pattern: None,  // Will be set by PersistentSubscriptionManager
            last_ack_seq: 0,
            last_ack_topic: None,
            created_at: now,
            last_activity: now,
        };
        
        let key = subscription_id.as_bytes();
        let value = bincode::serialize(&state)
            .map_err(|e| Error::Internal(format!("Failed to serialize subscription: {}", e)))?;
        
        self.subscriptions_tree
            .insert(key, value)
            .map_err(|e| Error::Internal(format!("Failed to create subscription: {}", e)))?;
        
        self.subscriptions_tree
            .flush_async()
            .await
            .map_err(|e| Error::Internal(format!("Failed to flush subscription: {}", e)))?;
        
        Ok(state)
    }

    /// Delete old messages based on retention policy
    pub async fn delete_old_messages(&self, topic: &str) -> Result<usize> {
        let metadata = {
            let cache = self.topic_metadata_cache.read().await;
            match cache.get(topic) {
                Some(m) => m.clone(),
                None => return Ok(0), // No metadata, nothing to delete
            }
        };
        
        if !metadata.retention_policy.has_limits() {
            return Ok(0); // No retention limits
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let prefix = format!("{}:", topic);
        let mut messages_to_delete = Vec::new();
        let mut messages_info = Vec::new();
        
        // Collect all messages for the topic
        for item in self.messages_tree.scan_prefix(prefix.as_bytes()) {
            let (key, value) = item.map_err(|e| Error::Internal(format!("Failed to read message: {}", e)))?;
            
            let message: PersistentMessage = bincode::deserialize(&value)
                .map_err(|e| Error::Internal(format!("Failed to deserialize message: {}", e)))?;
            
            messages_info.push((key.to_vec(), message));
        }
        
        let mut total_count = messages_info.len();
        let mut total_bytes: usize = messages_info.iter().map(|(_, m)| m.size_bytes).sum();
        
        // Apply retention policies
        for (key, message) in messages_info.iter() {
            let mut should_delete = false;
            
            // Check age
            if !metadata.retention_policy.should_retain_by_age(message.timestamp, now) {
                should_delete = true;
            }
            
            // Check count (delete oldest first if over limit)
            if let Some(max_count) = metadata.retention_policy.max_count {
                if total_count > max_count {
                    should_delete = true;
                    total_count -= 1;
                }
            }
            
            // Check size (delete oldest first if over limit)
            if let Some(max_bytes) = metadata.retention_policy.max_bytes {
                if total_bytes > max_bytes {
                    should_delete = true;
                    total_bytes = total_bytes.saturating_sub(message.size_bytes);
                }
            }
            
            if should_delete {
                messages_to_delete.push(key.clone());
            }
        }
        
        // Delete messages
        let deleted_count = messages_to_delete.len();
        for key in messages_to_delete {
            self.messages_tree
                .remove(&key)
                .map_err(|e| Error::Internal(format!("Failed to delete message: {}", e)))?;
        }
        
        if deleted_count > 0 {
            self.messages_tree
                .flush_async()
                .await
                .map_err(|e| Error::Internal(format!("Failed to flush after deletion: {}", e)))?;
            
            // Update metadata counts
            let mut updated_metadata = metadata;
            updated_metadata.message_count = updated_metadata.message_count.saturating_sub(deleted_count);
            updated_metadata.total_bytes = total_bytes;
            
            let metadata_key = topic.as_bytes();
            let metadata_value = bincode::serialize(&updated_metadata)
                .map_err(|e| Error::Internal(format!("Failed to serialize metadata: {}", e)))?;
            
            self.metadata_tree
                .insert(metadata_key, metadata_value)
                .map_err(|e| Error::Internal(format!("Failed to update metadata: {}", e)))?;
            
            self.metadata_tree
                .flush_async()
                .await
                .map_err(|e| Error::Internal(format!("Failed to flush metadata: {}", e)))?;
            
            // Update cache
            self.topic_metadata_cache.write().await.insert(topic.to_string(), updated_metadata);
        }
        
        Ok(deleted_count)
    }

    /// Get all topics with registered metadata
    pub async fn get_all_topics(&self) -> Result<Vec<String>> {
        let cache = self.topic_metadata_cache.read().await;
        Ok(cache.keys().cloned().collect())
    }

    /// Get topic metadata
    pub async fn get_topic_metadata(&self, topic: &str) -> Option<TopicMetadata> {
        let cache = self.topic_metadata_cache.read().await;
        cache.get(topic).cloned()
    }

    /// Delete subscription state
    pub async fn delete_subscription(&self, subscription_id: &str) -> Result<bool> {
        let key = subscription_id.as_bytes();
        let existed = self.subscriptions_tree
            .remove(key)
            .map_err(|e| Error::Internal(format!("Failed to delete subscription: {}", e)))?
            .is_some();
        
        if existed {
            self.subscriptions_tree
                .flush_async()
                .await
                .map_err(|e| Error::Internal(format!("Failed to flush after deletion: {}", e)))?;
        }
        
        Ok(existed)
    }

    /// Get all subscriptions
    pub async fn get_all_subscriptions(&self) -> Result<Vec<SubscriptionState>> {
        let mut subscriptions = Vec::new();
        
        for item in self.subscriptions_tree.iter() {
            let (_, value) = item.map_err(|e| Error::Internal(format!("Failed to read subscription: {}", e)))?;
            let state: SubscriptionState = bincode::deserialize(&value)
                .map_err(|e| Error::Internal(format!("Failed to deserialize subscription: {}", e)))?;
            subscriptions.push(state);
        }
        
        Ok(subscriptions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_store_and_retrieve_message() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = PersistentStorage::new(temp_dir.path()).unwrap();
        
        let data = serde_json::json!({"test": "data"});
        let seq = storage.store_message("test_topic", data.clone()).await.unwrap();
        
        assert_eq!(seq, 1);
        
        let messages = storage.get_messages_since("test_topic", 0).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].sequence_id, 1);
        assert_eq!(messages[0].data_as_value().unwrap(), data);
    }

    #[tokio::test]
    async fn test_multiple_messages() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = PersistentStorage::new(temp_dir.path()).unwrap();
        
        for i in 1..=5 {
            let data = serde_json::json!({"msg": i});
            let seq = storage.store_message("test", data).await.unwrap();
            assert_eq!(seq, i as u64);
        }
        
        let messages = storage.get_messages_since("test", 2).await.unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].sequence_id, 3);
        assert_eq!(messages[2].sequence_id, 5);
    }

    #[tokio::test]
    async fn test_subscription_state() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = PersistentStorage::new(temp_dir.path()).unwrap();
        
        let state = storage.create_subscription("sub1", "topic1").await.unwrap();
        assert_eq!(state.subscription_id, "sub1");
        assert_eq!(state.topic, "topic1");
        assert_eq!(state.last_ack_seq, 0);
        
        storage.update_subscription_position("sub1", 10).await.unwrap();
        
        let updated = storage.get_subscription_state("sub1").await.unwrap().unwrap();
        assert_eq!(updated.last_ack_seq, 10);
    }

    #[tokio::test]
    async fn test_retention_by_count() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = PersistentStorage::new(temp_dir.path()).unwrap();
        
        let policy = RetentionPolicy::by_count(3);
        storage.register_topic("test", policy).await.unwrap();
        
        // Store 5 messages
        for i in 1..=5 {
            storage.store_message("test", serde_json::json!({"msg": i})).await.unwrap();
        }
        
        // Apply retention
        let deleted = storage.delete_old_messages("test").await.unwrap();
        assert_eq!(deleted, 2);
        
        let messages = storage.get_messages_since("test", 0).await.unwrap();
        assert_eq!(messages.len(), 3);
    }
}

