//! Reconnection strategies for automatic reconnection
//!
//! This module provides configurable strategies for handling connection failures.
//! When the WebSocket connection drops, the strategy determines:
//! - How long to wait before attempting reconnection
//! - Whether to keep trying or give up
//!
//! # Built-in Strategies
//!
//! - **ExponentialBackoff**: Exponentially increasing delays (recommended)
//! - **FixedDelay**: Constant delay between attempts
//! - **NoReconnect**: Don't reconnect (fail immediately)
//!
//! # Custom Strategies
//!
//! Implement the `ReconnectionStrategy` trait to create custom behavior.
//!
//! # Examples
//!
//! ```rust
//! use jrow_client::ExponentialBackoff;
//! use std::time::Duration;
//!
//! // Default: 100ms to 30s, max 10 attempts, with jitter
//! let default = ExponentialBackoff::default();
//!
//! // Custom: 1s to 60s, unlimited attempts
//! let custom = ExponentialBackoff::new(
//!     Duration::from_secs(1),
//!     Duration::from_secs(60)
//! );
//! ```

use std::time::Duration;

/// Trait for reconnection strategies
///
/// Implementations control the reconnection behavior when the connection
/// is lost. The strategy is called repeatedly until either reconnection
/// succeeds or the strategy indicates giving up.
///
/// # State Management
///
/// The strategy maintains state across reconnection attempts (like current
/// attempt count). The `reset()` method is called after successful connection
/// to reset this state for future disconnects.
pub trait ReconnectionStrategy: Send + Sync {
    /// Returns the delay before the next reconnection attempt
    ///
    /// # Arguments
    ///
    /// * `attempt` - The current attempt number (0-indexed)
    ///
    /// # Returns
    ///
    /// - `Some(duration)`: Wait this long before attempting reconnection
    /// - `None`: Give up and don't attempt reconnection
    fn next_delay(&mut self, attempt: u32) -> Option<Duration>;

    /// Reset the strategy state after successful connection
    ///
    /// Called when reconnection succeeds, allowing the strategy to reset
    /// any accumulated state (counters, delays, etc.) for the next disconnect.
    fn reset(&mut self);
}

/// Exponential backoff reconnection strategy with optional jitter
pub struct ExponentialBackoff {
    min_delay: Duration,
    max_delay: Duration,
    max_attempts: Option<u32>,
    jitter: bool,
    current_attempt: u32,
}

impl ExponentialBackoff {
    /// Create a new exponential backoff strategy
    pub fn new(min_delay: Duration, max_delay: Duration) -> Self {
        Self {
            min_delay,
            max_delay,
            max_attempts: None,
            jitter: false,
            current_attempt: 0,
        }
    }

    /// Set the maximum number of attempts before giving up
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = Some(max_attempts);
        self
    }

    /// Enable jitter to prevent thundering herd
    pub fn with_jitter(mut self) -> Self {
        self.jitter = true;
        self
    }
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self::new(Duration::from_millis(100), Duration::from_secs(30))
            .with_max_attempts(10)
            .with_jitter()
    }
}

impl ReconnectionStrategy for ExponentialBackoff {
    fn next_delay(&mut self, attempt: u32) -> Option<Duration> {
        self.current_attempt = attempt;

        // Check if we've exceeded max attempts
        if let Some(max) = self.max_attempts {
            if attempt >= max {
                return None;
            }
        }

        // Calculate exponential backoff: min_delay * 2^attempt
        let base_delay = self.min_delay.as_millis() as u64 * 2u64.pow(attempt);
        let delay = std::cmp::min(base_delay, self.max_delay.as_millis() as u64);

        let mut final_delay = Duration::from_millis(delay);

        // Add jitter if enabled (random 0-25% of delay)
        if self.jitter {
            use rand::Rng;
            let jitter_ms = rand::thread_rng().gen_range(0..=(delay / 4));
            final_delay = Duration::from_millis(delay + jitter_ms);
        }

        Some(final_delay)
    }

    fn reset(&mut self) {
        self.current_attempt = 0;
    }
}

/// Fixed delay reconnection strategy
pub struct FixedDelay {
    delay: Duration,
    max_attempts: Option<u32>,
}

impl FixedDelay {
    /// Create a new fixed delay strategy
    pub fn new(delay: Duration) -> Self {
        Self {
            delay,
            max_attempts: None,
        }
    }

    /// Set the maximum number of attempts before giving up
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = Some(max_attempts);
        self
    }
}

impl ReconnectionStrategy for FixedDelay {
    fn next_delay(&mut self, attempt: u32) -> Option<Duration> {
        if let Some(max) = self.max_attempts {
            if attempt >= max {
                return None;
            }
        }
        Some(self.delay)
    }

    fn reset(&mut self) {
        // No state to reset for fixed delay
    }
}

/// Custom reconnection strategy that never reconnects
pub struct NoReconnect;

impl ReconnectionStrategy for NoReconnect {
    fn next_delay(&mut self, _attempt: u32) -> Option<Duration> {
        None
    }

    fn reset(&mut self) {
        // No state to reset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff_basic() {
        let mut strategy = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(10),
        )
        .with_max_attempts(5);

        // First attempt: 100ms
        let delay1 = strategy.next_delay(0).unwrap();
        assert_eq!(delay1, Duration::from_millis(100));

        // Second attempt: 200ms
        let delay2 = strategy.next_delay(1).unwrap();
        assert_eq!(delay2, Duration::from_millis(200));

        // Third attempt: 400ms
        let delay3 = strategy.next_delay(2).unwrap();
        assert_eq!(delay3, Duration::from_millis(400));
    }

    #[test]
    fn test_exponential_backoff_max_delay() {
        let mut strategy = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(1),
        );

        // Should cap at max_delay (1 second = 1000ms)
        let delay = strategy.next_delay(10).unwrap();
        assert_eq!(delay, Duration::from_millis(1000));
    }

    #[test]
    fn test_exponential_backoff_max_attempts() {
        let mut strategy = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(10),
        )
        .with_max_attempts(3);

        assert!(strategy.next_delay(0).is_some());
        assert!(strategy.next_delay(1).is_some());
        assert!(strategy.next_delay(2).is_some());
        assert!(strategy.next_delay(3).is_none()); // Exceeded max attempts
    }

    #[test]
    fn test_exponential_backoff_reset() {
        let mut strategy = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(10),
        );

        strategy.next_delay(5);
        assert_eq!(strategy.current_attempt, 5);

        strategy.reset();
        assert_eq!(strategy.current_attempt, 0);
    }

    #[test]
    fn test_fixed_delay() {
        let mut strategy = FixedDelay::new(Duration::from_secs(1))
            .with_max_attempts(3);

        assert_eq!(strategy.next_delay(0).unwrap(), Duration::from_secs(1));
        assert_eq!(strategy.next_delay(1).unwrap(), Duration::from_secs(1));
        assert_eq!(strategy.next_delay(2).unwrap(), Duration::from_secs(1));
        assert!(strategy.next_delay(3).is_none());
    }

    #[test]
    fn test_no_reconnect() {
        let mut strategy = NoReconnect;
        assert!(strategy.next_delay(0).is_none());
        assert!(strategy.next_delay(1).is_none());
    }

    #[test]
    fn test_exponential_backoff_jitter() {
        let mut strategy = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(10),
        )
        .with_jitter();

        // With jitter, delays should vary slightly
        let delay1 = strategy.next_delay(0).unwrap();
        // Should be between 100ms and 125ms (100 + 25% jitter)
        assert!(delay1 >= Duration::from_millis(100));
        assert!(delay1 <= Duration::from_millis(125));
    }
}



