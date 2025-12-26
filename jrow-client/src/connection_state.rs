//! Connection state management
//!
//! This module tracks the WebSocket connection lifecycle and coordinates
//! reconnection attempts when the connection is lost.
//!
//! # Connection States
//!
//! - **Disconnected**: Initial state, not connected
//! - **Connecting**: Attempting to establish connection
//! - **Connected**: Successfully connected and operational
//! - **Reconnecting**: Connection lost, attempting to reconnect
//! - **Failed**: Reconnection attempts exhausted, gave up
//!
//! # State Transitions
//!
//! ```text
//! Disconnected → Connecting → Connected
//!                      ↓           ↓
//!                   Failed ← Reconnecting
//! ```
//!
//! # Reconnection Logic
//!
//! When connected state transitions to disconnected:
//! 1. Enter Reconnecting state
//! 2. Consult ReconnectionStrategy for delay
//! 3. Wait the specified duration
//! 4. Attempt connection
//! 5. On success: return to Connected (strategy resets)
//! 6. On failure: repeat from step 2 or enter Failed if strategy gives up

use crate::reconnect::ReconnectionStrategy;
use jrow_core::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Attempting to connect
    Connecting,
    /// Successfully connected
    Connected,
    /// Reconnecting after disconnection
    Reconnecting { attempt: u32 },
    /// Failed to reconnect (gave up)
    Failed,
}

/// Manages connection state and reconnection logic
pub struct ConnectionManager {
    state: Arc<RwLock<ConnectionState>>,
    strategy: Arc<RwLock<Box<dyn ReconnectionStrategy>>>,
    url: String,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(url: String, strategy: Box<dyn ReconnectionStrategy>) -> Self {
        Self {
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            strategy: Arc::new(RwLock::new(strategy)),
            url,
        }
    }

    /// Get the current connection state
    pub async fn state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    /// Set the connection state
    pub async fn set_state(&self, new_state: ConnectionState) {
        *self.state.write().await = new_state;
    }

    /// Get the connection URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Transition to connecting state
    pub async fn connecting(&self) {
        self.set_state(ConnectionState::Connecting).await;
    }

    /// Transition to connected state
    pub async fn connected(&self) {
        self.set_state(ConnectionState::Connected).await;
        // Reset reconnection strategy on successful connection
        self.strategy.write().await.reset();
    }

    /// Transition to disconnected state
    pub async fn disconnected(&self) {
        self.set_state(ConnectionState::Disconnected).await;
    }

    /// Start reconnection attempts
    pub async fn start_reconnecting(&self) -> Result<(), Error> {
        self.set_state(ConnectionState::Reconnecting { attempt: 0 })
            .await;
        Ok(())
    }

    /// Get the next reconnection delay
    /// Returns None if reconnection should be abandoned
    pub async fn next_reconnect_delay(&self) -> Option<std::time::Duration> {
        let current_state = self.state().await;
        
        let attempt = match current_state {
            ConnectionState::Reconnecting { attempt } => attempt,
            _ => 0,
        };

        let delay = self.strategy.write().await.next_delay(attempt);

        if delay.is_some() {
            // Update state with incremented attempt
            self.set_state(ConnectionState::Reconnecting {
                attempt: attempt + 1,
            })
            .await;
        } else {
            // No more attempts, mark as failed
            self.set_state(ConnectionState::Failed).await;
        }

        delay
    }

    /// Check if reconnection is enabled
    pub async fn should_reconnect(&self) -> bool {
        matches!(
            self.state().await,
            ConnectionState::Reconnecting { .. } | ConnectionState::Disconnected
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconnect::ExponentialBackoff;
    use std::time::Duration;

    #[tokio::test]
    async fn test_connection_state_transitions() {
        let strategy = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(10),
        );
        let manager = ConnectionManager::new("ws://localhost:8080".to_string(), Box::new(strategy));

        assert_eq!(manager.state().await, ConnectionState::Disconnected);

        manager.connecting().await;
        assert_eq!(manager.state().await, ConnectionState::Connecting);

        manager.connected().await;
        assert_eq!(manager.state().await, ConnectionState::Connected);

        manager.disconnected().await;
        assert_eq!(manager.state().await, ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_reconnection_attempts() {
        let strategy = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(10),
        )
        .with_max_attempts(3);
        
        let manager = ConnectionManager::new("ws://localhost:8080".to_string(), Box::new(strategy));

        manager.start_reconnecting().await.unwrap();
        assert_eq!(
            manager.state().await,
            ConnectionState::Reconnecting { attempt: 0 }
        );

        // First attempt
        let delay1 = manager.next_reconnect_delay().await;
        assert!(delay1.is_some());
        assert_eq!(
            manager.state().await,
            ConnectionState::Reconnecting { attempt: 1 }
        );

        // Second attempt
        let delay2 = manager.next_reconnect_delay().await;
        assert!(delay2.is_some());
        assert_eq!(
            manager.state().await,
            ConnectionState::Reconnecting { attempt: 2 }
        );

        // Third attempt
        let delay3 = manager.next_reconnect_delay().await;
        assert!(delay3.is_some());
        assert_eq!(
            manager.state().await,
            ConnectionState::Reconnecting { attempt: 3 }
        );

        // Fourth attempt should fail (max attempts reached)
        let delay4 = manager.next_reconnect_delay().await;
        assert!(delay4.is_none());
        assert_eq!(manager.state().await, ConnectionState::Failed);
    }

    #[tokio::test]
    async fn test_strategy_reset_on_connect() {
        let strategy = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(10),
        );
        
        let manager = ConnectionManager::new("ws://localhost:8080".to_string(), Box::new(strategy));

        manager.start_reconnecting().await.unwrap();
        manager.next_reconnect_delay().await;
        manager.next_reconnect_delay().await;

        // After successful connection, strategy should reset
        manager.connected().await;
        
        // Start reconnecting again
        manager.start_reconnecting().await.unwrap();
        assert_eq!(
            manager.state().await,
            ConnectionState::Reconnecting { attempt: 0 }
        );
    }
}



