//! Topic filtering with NATS-style pattern support
//!
//! This module implements pattern-based subscriptions using NATS-style wildcards:
//! - `*` matches exactly one token
//! - `>` matches one or more trailing tokens  
//!
//! # Why Pattern Subscriptions?
//!
//! Pattern subscriptions allow clients to subscribe to multiple related topics
//! with a single subscription. For example:
//! - `orders.*` matches `orders.created`, `orders.updated`, etc.
//! - `events.>` matches any topic starting with `events.`
//!
//! This is more efficient than maintaining multiple exact subscriptions and
//! simplifies client code.
//!
//! # Examples
//!
//! ```rust
//! use jrow_server::TopicFilter;
//!
//! // Exact match
//! let exact = TopicFilter::new("users.login").unwrap();
//! assert!(exact.matches("users.login"));
//! assert!(!exact.matches("users.logout"));
//!
//! // Single wildcard
//! let pattern = TopicFilter::new("events.*.created").unwrap();
//! assert!(pattern.matches("events.user.created"));
//! assert!(pattern.matches("events.order.created"));
//! assert!(!pattern.matches("events.user.updated"));
//!
//! // Multi wildcard
//! let multi = TopicFilter::new("logs.>").unwrap();
//! assert!(multi.matches("logs.error"));
//! assert!(multi.matches("logs.error.database"));
//! ```

use crate::nats_pattern::NatsPattern;
use std::collections::HashMap;

/// Topic filter for matching subscription patterns
///
/// Encapsulates either an exact topic string or a NATS-style pattern.
/// The type is determined automatically based on wildcard presence.
///
/// # Variants
///
/// - **Exact**: Direct string comparison (no wildcards)
/// - **Pattern**: NATS pattern matching with `*` or `>` wildcards
#[derive(Debug, Clone)]
pub enum TopicFilter {
    /// Exact topic match - fast string comparison
    Exact(String),
    
    /// NATS-style pattern match with wildcards
    ///
    /// Stores both the compiled pattern (for matching) and the original
    /// string (for display/comparison).
    Pattern { 
        /// Compiled pattern matcher
        pattern: NatsPattern, 
        /// Original pattern string
        original: String 
    },
}

/// Error type for topic filter operations
#[derive(Debug, thiserror::Error)]
pub enum FilterError {
    #[error("Invalid NATS pattern: {0}")]
    InvalidPattern(String),
}

impl TopicFilter {
    /// Create a new topic filter from a string
    /// If the string contains NATS wildcards (* or >), it's treated as a pattern
    pub fn new(topic: impl Into<String>) -> Result<Self, FilterError> {
        let topic = topic.into();
        
        // Check if it contains NATS wildcards
        if topic.contains('*') || topic.contains('>') {
            match NatsPattern::new(&topic) {
                Ok(pattern) => Ok(TopicFilter::Pattern { 
                    pattern, 
                    original: topic 
                }),
                Err(e) => Err(FilterError::InvalidPattern(e.to_string())),
            }
        } else {
            Ok(TopicFilter::Exact(topic))
        }
    }

    /// Check if a topic matches this filter
    pub fn matches(&self, topic: &str) -> bool {
        match self {
            TopicFilter::Exact(exact) => exact == topic,
            TopicFilter::Pattern { pattern, .. } => pattern.matches(topic),
        }
    }

    /// Get the original pattern string
    pub fn as_str(&self) -> &str {
        match self {
            TopicFilter::Exact(s) => s,
            TopicFilter::Pattern { original, .. } => original,
        }
    }
}

/// Manages topic subscriptions with pattern matching support
#[derive(Debug)]
pub struct FilteredSubscriptionManager {
    /// Map of connection ID to their subscription filters
    subscriptions: HashMap<u64, Vec<TopicFilter>>,
}

impl FilteredSubscriptionManager {
    /// Create a new filtered subscription manager
    pub fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
        }
    }

    /// Subscribe a connection to a topic pattern
    pub fn subscribe(&mut self, conn_id: u64, pattern: TopicFilter) {
        self.subscriptions
            .entry(conn_id)
            .or_insert_with(Vec::new)
            .push(pattern);
    }

    /// Unsubscribe a connection from a specific pattern
    pub fn unsubscribe(&mut self, conn_id: u64, pattern: &str) -> bool {
        if let Some(filters) = self.subscriptions.get_mut(&conn_id) {
            let before_len = filters.len();
            filters.retain(|f| f.as_str() != pattern);
            filters.len() < before_len
        } else {
            false
        }
    }

    /// Get all connection IDs that match a given topic
    pub fn get_subscribers(&self, topic: &str) -> Vec<u64> {
        let mut subscribers = Vec::new();
        
        for (&conn_id, filters) in &self.subscriptions {
            for filter in filters {
                if filter.matches(topic) {
                    subscribers.push(conn_id);
                    break; // Only add each connection once
                }
            }
        }
        
        subscribers
    }

    /// Get all subscribers with their matching patterns for a given topic
    /// Returns a vector of (connection_id, pattern_string) tuples
    pub fn get_subscribers_with_patterns(&self, topic: &str) -> Vec<(u64, String)> {
        let mut result = Vec::new();
        
        for (&conn_id, filters) in &self.subscriptions {
            for filter in filters {
                if filter.matches(topic) {
                    result.push((conn_id, filter.as_str().to_string()));
                }
            }
        }
        
        result
    }

    /// Remove all subscriptions for a connection
    pub fn remove_connection(&mut self, conn_id: u64) {
        self.subscriptions.remove(&conn_id);
    }

    /// Get all patterns for a connection
    pub fn get_patterns(&self, conn_id: u64) -> Vec<String> {
        self.subscriptions
            .get(&conn_id)
            .map(|filters| filters.iter().map(|f| f.as_str().to_string()).collect())
            .unwrap_or_default()
    }

    /// Get total number of subscriptions across all connections
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.values().map(|v| v.len()).sum()
    }
}

impl Default for FilteredSubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_filter() {
        let filter = TopicFilter::new("events.user.login").unwrap();
        assert!(filter.matches("events.user.login"));
        assert!(!filter.matches("events.user.logout"));
        assert!(!filter.matches("events.admin.login"));
    }

    #[test]
    fn test_single_wildcard() {
        let filter = TopicFilter::new("events.user.*").unwrap();
        assert!(filter.matches("events.user.login"));
        assert!(filter.matches("events.user.logout"));
        assert!(filter.matches("events.user.anything"));
        assert!(!filter.matches("events.admin.login"));
        assert!(!filter.matches("events.user")); // * requires exactly one token
        assert!(!filter.matches("events.user.login.success")); // * only matches one token
    }

    #[test]
    fn test_multiple_single_wildcards() {
        let filter = TopicFilter::new("events.*.login").unwrap();
        assert!(filter.matches("events.user.login"));
        assert!(filter.matches("events.admin.login"));
        assert!(!filter.matches("events.user.logout"));
        assert!(!filter.matches("events.super.admin.login")); // * only matches one token
    }

    #[test]
    fn test_multi_wildcard() {
        let filter = TopicFilter::new("events.>").unwrap();
        assert!(filter.matches("events.user"));
        assert!(filter.matches("events.user.login"));
        assert!(filter.matches("events.user.login.success"));
        assert!(!filter.matches("events")); // > requires at least one token after
        assert!(!filter.matches("other.event"));
    }

    #[test]
    fn test_multi_wildcard_with_prefix() {
        let filter = TopicFilter::new("orders.new.>").unwrap();
        assert!(filter.matches("orders.new.express"));
        assert!(filter.matches("orders.new.express.priority"));
        assert!(!filter.matches("orders.new")); // > requires at least one token
        assert!(!filter.matches("orders.shipped.express"));
    }

    #[test]
    fn test_subscription_manager_exact() {
        let mut manager = FilteredSubscriptionManager::new();
        
        let filter = TopicFilter::new("topic1").unwrap();
        manager.subscribe(1, filter);
        
        assert_eq!(manager.get_subscribers("topic1"), vec![1]);
        assert_eq!(manager.get_subscribers("topic2"), Vec::<u64>::new());
    }

    #[test]
    fn test_subscription_manager_pattern_single_wildcard() {
        let mut manager = FilteredSubscriptionManager::new();
        
        let filter = TopicFilter::new("orders.*").unwrap();
        manager.subscribe(1, filter);
        
        assert_eq!(manager.get_subscribers("orders.new"), vec![1]);
        assert_eq!(manager.get_subscribers("orders.shipped"), vec![1]);
        assert_eq!(manager.get_subscribers("orders.new.express"), Vec::<u64>::new());
        assert_eq!(manager.get_subscribers("orders"), Vec::<u64>::new());
    }

    #[test]
    fn test_subscription_manager_pattern_multi_wildcard() {
        let mut manager = FilteredSubscriptionManager::new();
        
        let filter = TopicFilter::new("events.>").unwrap();
        manager.subscribe(1, filter);
        
        assert_eq!(manager.get_subscribers("events.user"), vec![1]);
        assert_eq!(manager.get_subscribers("events.user.login"), vec![1]);
        assert_eq!(manager.get_subscribers("events.user.login.success"), vec![1]);
        assert_eq!(manager.get_subscribers("events"), Vec::<u64>::new());
    }

    #[test]
    fn test_subscription_manager_pattern() {
        let mut manager = FilteredSubscriptionManager::new();
        
        let filter1 = TopicFilter::new("events.*").unwrap();
        let filter2 = TopicFilter::new("logs.*").unwrap();
        
        manager.subscribe(1, filter1);
        manager.subscribe(2, filter2);
        
        assert_eq!(manager.get_subscribers("events.login"), vec![1]);
        assert_eq!(manager.get_subscribers("logs.error"), vec![2]);
        assert_eq!(manager.get_subscribers("other.topic"), Vec::<u64>::new());
    }

    #[test]
    fn test_subscription_manager_multiple_patterns() {
        let mut manager = FilteredSubscriptionManager::new();
        
        let filter1 = TopicFilter::new("events.*").unwrap();
        let filter2 = TopicFilter::new("*.login").unwrap();
        
        manager.subscribe(1, filter1);
        manager.subscribe(1, filter2);
        
        // Should only return connection once even though both patterns match
        assert_eq!(manager.get_subscribers("events.login"), vec![1]);
    }

    #[test]
    fn test_unsubscribe() {
        let mut manager = FilteredSubscriptionManager::new();
        
        let filter = TopicFilter::new("events.*").unwrap();
        manager.subscribe(1, filter);
        
        assert_eq!(manager.get_subscribers("events.login"), vec![1]);
        
        assert!(manager.unsubscribe(1, "events.*"));
        assert_eq!(manager.get_subscribers("events.login"), Vec::<u64>::new());
        
        // Unsubscribing again should return false
        assert!(!manager.unsubscribe(1, "events.*"));
    }

    #[test]
    fn test_remove_connection() {
        let mut manager = FilteredSubscriptionManager::new();
        
        let filter1 = TopicFilter::new("events.*").unwrap();
        let filter2 = TopicFilter::new("logs.*").unwrap();
        
        manager.subscribe(1, filter1);
        manager.subscribe(1, filter2);
        
        assert_eq!(manager.get_subscribers("events.login"), vec![1]);
        assert_eq!(manager.get_subscribers("logs.error"), vec![1]);
        
        manager.remove_connection(1);
        
        assert_eq!(manager.get_subscribers("events.login"), Vec::<u64>::new());
        assert_eq!(manager.get_subscribers("logs.error"), Vec::<u64>::new());
    }

    #[test]
    fn test_get_patterns() {
        let mut manager = FilteredSubscriptionManager::new();
        
        let filter1 = TopicFilter::new("events.*").unwrap();
        let filter2 = TopicFilter::new("logs.*").unwrap();
        
        manager.subscribe(1, filter1);
        manager.subscribe(1, filter2);
        
        let patterns = manager.get_patterns(1);
        assert_eq!(patterns.len(), 2);
        assert!(patterns.contains(&"events.*".to_string()));
        assert!(patterns.contains(&"logs.*".to_string()));
    }

    #[test]
    fn test_subscription_count() {
        let mut manager = FilteredSubscriptionManager::new();
        
        let filter1 = TopicFilter::new("events.*").unwrap();
        let filter2 = TopicFilter::new("logs.*").unwrap();
        let filter3 = TopicFilter::new("alerts.*").unwrap();
        
        manager.subscribe(1, filter1);
        manager.subscribe(1, filter2);
        manager.subscribe(2, filter3);
        
        assert_eq!(manager.subscription_count(), 3);
    }

    #[test]
    fn test_complex_pattern() {
        let filter = TopicFilter::new("events.*.user.*.action").unwrap();
        assert!(filter.matches("events.app.user.123.action"));
        assert!(filter.matches("events.web.user.456.action"));
        assert!(!filter.matches("events.app.admin.123.action"));
    }
}


