//! Retention policy configuration and enforcement for persistent messages
//!
//! Retention policies control how long messages are kept in persistent storage.
//! Without retention, storage would grow indefinitely. Policies allow automatic
//! cleanup based on:
//! - **Age**: Delete messages older than N seconds/days
//! - **Count**: Keep only the latest N messages
//! - **Size**: Limit total storage to N bytes
//!
//! # Policy Evaluation
//!
//! Policies are evaluated periodically by a background task. When any limit
//! is exceeded, the oldest messages are deleted until all limits are satisfied.
//!
//! # Multiple Criteria
//!
//! Policies can specify multiple criteria (e.g., max age AND max count).
//! Messages are retained only if they satisfy ALL specified criteria.
//!
//! # Examples
//!
//! ```rust
//! use jrow_server::RetentionPolicy;
//! use std::time::Duration;
//!
//! // Keep messages for 7 days
//! let time_based = RetentionPolicy::by_age(Duration::from_secs(7 * 86400));
//!
//! // Keep last 1000 messages
//! let count_based = RetentionPolicy::by_count(1000);
//!
//! // Limit to 100MB
//! let size_based = RetentionPolicy::by_size(100 * 1024 * 1024);
//!
//! // Combined: 7 days OR 1000 messages, whichever is smaller
//! let combined = RetentionPolicy {
//!     max_age: Some(Duration::from_secs(7 * 86400)),
//!     max_count: Some(1000),
//!     max_bytes: None,
//! };
//! ```

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Retention policy for a topic
///
/// Defines limits on how long messages should be retained. Messages exceeding
/// any limit are eligible for deletion. Setting all fields to None creates
/// an unlimited policy (not recommended for production).
///
/// # Evaluation
///
/// - All criteria are evaluated independently
/// - Messages are kept if they satisfy ALL specified limits
/// - Oldest messages are deleted first when limits are exceeded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Maximum age of messages (None = no age limit)
    /// Messages older than this duration will be deleted
    pub max_age: Option<Duration>,
    
    /// Maximum number of messages to keep (None = no count limit)
    /// If more messages exist, oldest are deleted
    pub max_count: Option<usize>,
    
    /// Maximum total size in bytes (None = no size limit)
    /// If total size exceeds this, oldest messages deleted until under limit
    pub max_bytes: Option<usize>,
}

impl RetentionPolicy {
    /// Create a new retention policy with no limits
    pub fn unlimited() -> Self {
        Self {
            max_age: None,
            max_count: None,
            max_bytes: None,
        }
    }

    /// Create a policy with only time-based retention
    pub fn by_age(duration: Duration) -> Self {
        Self {
            max_age: Some(duration),
            max_count: None,
            max_bytes: None,
        }
    }

    /// Create a policy with only count-based retention
    pub fn by_count(count: usize) -> Self {
        Self {
            max_age: None,
            max_count: Some(count),
            max_bytes: None,
        }
    }

    /// Create a policy with only size-based retention
    pub fn by_size(bytes: usize) -> Self {
        Self {
            max_age: None,
            max_count: None,
            max_bytes: Some(bytes),
        }
    }

    /// Check if a message should be retained based on age
    pub fn should_retain_by_age(&self, message_timestamp: u64, current_timestamp: u64) -> bool {
        if let Some(max_age) = self.max_age {
            let age_secs = current_timestamp.saturating_sub(message_timestamp);
            age_secs <= max_age.as_secs()
        } else {
            true
        }
    }

    /// Check if we're within count limits
    pub fn should_retain_by_count(&self, current_count: usize) -> bool {
        if let Some(max_count) = self.max_count {
            current_count <= max_count
        } else {
            true
        }
    }

    /// Check if we're within size limits
    pub fn should_retain_by_size(&self, current_bytes: usize) -> bool {
        if let Some(max_bytes) = self.max_bytes {
            current_bytes <= max_bytes
        } else {
            true
        }
    }

    /// Check if any retention limit is configured
    pub fn has_limits(&self) -> bool {
        self.max_age.is_some() || self.max_count.is_some() || self.max_bytes.is_some()
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::unlimited()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unlimited_policy() {
        let policy = RetentionPolicy::unlimited();
        assert!(!policy.has_limits());
        assert!(policy.should_retain_by_age(0, u64::MAX));
        assert!(policy.should_retain_by_count(usize::MAX));
        assert!(policy.should_retain_by_size(usize::MAX));
    }

    #[test]
    fn test_age_based_retention() {
        let policy = RetentionPolicy::by_age(Duration::from_secs(3600)); // 1 hour
        
        let now = 10000;
        let one_hour_ago = now - 3600;
        let two_hours_ago = now - 7200;
        
        assert!(policy.should_retain_by_age(one_hour_ago, now));
        assert!(policy.should_retain_by_age(now, now));
        assert!(!policy.should_retain_by_age(two_hours_ago, now));
    }

    #[test]
    fn test_count_based_retention() {
        let policy = RetentionPolicy::by_count(100);
        
        assert!(policy.should_retain_by_count(50));
        assert!(policy.should_retain_by_count(100));
        assert!(!policy.should_retain_by_count(101));
    }

    #[test]
    fn test_size_based_retention() {
        let policy = RetentionPolicy::by_size(1024 * 1024); // 1MB
        
        assert!(policy.should_retain_by_size(512 * 1024));
        assert!(policy.should_retain_by_size(1024 * 1024));
        assert!(!policy.should_retain_by_size(2 * 1024 * 1024));
    }

    #[test]
    fn test_combined_policy() {
        let policy = RetentionPolicy {
            max_age: Some(Duration::from_secs(3600)),
            max_count: Some(1000),
            max_bytes: Some(10 * 1024 * 1024),
        };
        
        assert!(policy.has_limits());
    }
}



