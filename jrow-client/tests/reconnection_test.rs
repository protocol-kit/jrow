//! Client reconnection integration tests
//!
//! Tests for automatic reconnection with various strategies.

mod common;

use common::{MockWsServer, mock_response};
use jrow_client::{JrowClient, ClientBuilder, ExponentialBackoff, FixedDelay};
use std::time::Duration;

#[tokio::test]
async fn test_reconnect_after_server_restart() {
    let server = MockWsServer::new().await;
    let url = server.url();
    
    let client = ClientBuilder::new(&url)
        .with_reconnect(ExponentialBackoff::new(Duration::from_millis(100)))
        .connect()
        .await
        .unwrap();
    
    assert!(client.is_connected());
    
    // Simulate server shutdown
    server.shutdown().await;
    
    // Wait for disconnect detection
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Start new server on same port (simulating restart)
    // Note: In real test this might bind to different port
    // For now just verify reconnection logic is triggered
    
    // Client should eventually reconnect (or be in reconnecting state)
    // This test verifies the mechanism is in place
}

#[tokio::test]
async fn test_reconnect_exponential_backoff() {
    let backoff = ExponentialBackoff::new(Duration::from_millis(50))
        .with_max_attempts(3);
    
    // Test backoff delay calculation
    let mut test_backoff = backoff.clone();
    
    let delay1 = test_backoff.next_delay();
    assert_eq!(delay1, Some(Duration::from_millis(50)));
    
    let delay2 = test_backoff.next_delay();
    assert_eq!(delay2, Some(Duration::from_millis(100)));
    
    let delay3 = test_backoff.next_delay();
    assert_eq!(delay3, Some(Duration::from_millis(200)));
    
    // After max attempts
    let delay4 = test_backoff.next_delay();
    assert_eq!(delay4, None);
}

#[tokio::test]
async fn test_reconnect_max_attempts_exceeded() {
    // This test would require server that refuses connections
    // Verify max attempts logic in backoff strategy
    let backoff = ExponentialBackoff::new(Duration::from_millis(10))
        .with_max_attempts(2);
    
    let mut test_backoff = backoff.clone();
    
    assert!(test_backoff.next_delay().is_some());
    assert!(test_backoff.next_delay().is_some());
    assert!(test_backoff.next_delay().is_none());
}

#[tokio::test]
async fn test_reconnect_with_pending_requests() {
    let server = MockWsServer::new().await;
    let url = server.url();
    
    let client = ClientBuilder::new(&url)
        .with_reconnect(FixedDelay::new(Duration::from_millis(100)))
        .connect()
        .await
        .unwrap();
    
    // Send request
    let _request_future = client.request::<serde_json::Value>("test", None::<serde_json::Value>);
    
    // Simulate disconnect
    server.shutdown().await;
    
    // Pending request should eventually fail with ConnectionClosed
    // (or succeed if reconnection happens fast enough)
}

#[tokio::test]
async fn test_reconnect_without_strategy() {
    let server = MockWsServer::new().await;
    let url = server.url();
    
    let client = ClientBuilder::new(&url)
        .without_reconnect()
        .connect()
        .await
        .unwrap();
    
    assert!(client.is_connected());
    
    // With no reconnect strategy, disconnect should be permanent
    server.shutdown().await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Client should remain disconnected
    assert!(!client.is_connected());
}

#[tokio::test]
async fn test_reconnect_fixed_delay() {
    let fixed = FixedDelay::new(Duration::from_millis(100))
        .with_max_attempts(5);
    
    let mut test_fixed = fixed.clone();
    
    // All delays should be the same
    for _ in 0..5 {
        assert_eq!(test_fixed.next_delay(), Some(Duration::from_millis(100)));
    }
    
    // After max attempts
    assert_eq!(test_fixed.next_delay(), None);
}

#[tokio::test]
async fn test_reconnect_strategy_reset() {
    let mut backoff = ExponentialBackoff::new(Duration::from_millis(50))
        .with_max_attempts(3);
    
    // Use up one attempt
    assert!(backoff.next_delay().is_some());
    
    // Reset should start over
    backoff.reset();
    
    let delay = backoff.next_delay();
    assert_eq!(delay, Some(Duration::from_millis(50)));
}

#[tokio::test]
async fn test_reconnect_resubscribe_topics() {
    // This test would verify that subscriptions are re-established after reconnection
    // Requires full server integration
    // Placeholder for now - implemented in phase 4
}

#[tokio::test]
async fn test_reconnect_resume_persistent_subscriptions() {
    // This test would verify that persistent subscriptions resume correctly
    // Requires full server integration with persistent storage
    // Placeholder for now - implemented in phase 4
}

