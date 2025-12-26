//! Background task for enforcing retention policies
//!
//! This module provides a background task that periodically enforces
//! retention policies on all topics with persistent storage.
//!
//! # Task Lifecycle
//!
//! The retention task:
//! 1. Starts when the server is built (if persistent storage enabled)
//! 2. Runs at a configured interval (default: 60 seconds)
//! 3. Checks each topic's retention policy
//! 4. Deletes messages exceeding the limits
//! 5. Continues until server shutdown
//!
//! # Graceful Shutdown
//!
//! The task listens for a shutdown signal and exits cleanly when
//! the server is dropped.
//!
//! # Error Handling
//!
//! Errors during retention enforcement are logged but don't stop
//! the task. This ensures one topic's issues don't affect others.
//!
//! # Performance
//!
//! The task processes topics sequentially to avoid overwhelming
//! the storage system. For many topics, consider tuning the interval.

use crate::persistent_storage::PersistentStorage;
use jrow_core::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

/// Background task that periodically enforces retention policies
pub async fn run_retention_task(
    storage: Arc<PersistentStorage>,
    interval: Duration,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    let mut interval_timer = time::interval(interval);
    
    tracing::info!(
        interval_secs = interval.as_secs(),
        "Starting retention enforcement task"
    );
    
    loop {
        tokio::select! {
            _ = interval_timer.tick() => {
                if let Err(e) = enforce_retention(&storage).await {
                    tracing::error!(error = %e, "Error enforcing retention policies");
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::info!("Retention task shutting down");
                    break;
                }
            }
        }
    }
}

/// Enforce retention policies for all topics
async fn enforce_retention(storage: &PersistentStorage) -> Result<()> {
    let topics = storage.get_all_topics().await?;
    
    let mut total_deleted = 0;
    
    for topic in topics {
        match storage.delete_old_messages(&topic).await {
            Ok(deleted) => {
                if deleted > 0 {
                    tracing::info!(
                        topic = %topic,
                        deleted_count = deleted,
                        "Enforced retention policy"
                    );
                    total_deleted += deleted;
                }
            }
            Err(e) => {
                tracing::error!(
                    topic = %topic,
                    error = %e,
                    "Failed to enforce retention policy"
                );
            }
        }
    }
    
    if total_deleted > 0 {
        tracing::debug!(total_deleted = total_deleted, "Retention enforcement completed");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PersistentStorage, RetentionPolicy};

    #[tokio::test]
    async fn test_retention_task_basic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(PersistentStorage::new(temp_dir.path()).unwrap());
        
        // Register topic with count limit
        storage
            .register_topic("test", RetentionPolicy::by_count(2))
            .await
            .unwrap();
        
        // Store 5 messages
        for i in 1..=5 {
            storage
                .store_message("test", serde_json::json!({"msg": i}))
                .await
                .unwrap();
        }
        
        // Enforce retention
        enforce_retention(&storage).await.unwrap();
        
        // Should have only 2 messages left
        let messages = storage.get_messages_since("test", 0).await.unwrap();
        assert_eq!(messages.len(), 2);
    }
}



