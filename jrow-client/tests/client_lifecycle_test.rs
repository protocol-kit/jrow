//! Client lifecycle integration tests
//!
//! Tests for client connection, disconnection, and state management.

mod common;

use common::MockWsServer;
use jrow_client::JrowClient;
use jrow_core::Error;

#[tokio::test]
async fn test_client_connect_success() {
    let server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await;
    
    assert!(client.is_ok());
    let client = client.unwrap();
    assert!(client.is_connected());
    
    server.shutdown().await;
}

#[tokio::test]
async fn test_client_connect_invalid_url() {
    // Test connection to non-existent server
    let result = JrowClient::connect("ws://localhost:99999").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_client_disconnect_graceful() {
    let server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    assert!(client.is_connected());
    
    // Disconnect the client
    client.disconnect().await;
    
    // Client should no longer be connected
    assert!(!client.is_connected());
    
    server.shutdown().await;
}

#[tokio::test]
async fn test_client_disconnect_while_pending_requests() {
    let server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    // Send a request but don't wait for response
    let request_future = client.request::<serde_json::Value>("test", None::<serde_json::Value>);
    
    // Disconnect immediately
    client.disconnect().await;
    
    // The pending request should fail
    let result = request_future.await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        Error::ConnectionClosed => {}, // Expected
        e => panic!("Expected ConnectionClosed error, got: {:?}", e),
    }
    
    server.shutdown().await;
}

#[tokio::test]
async fn test_client_multiple_connects() {
    let server = MockWsServer::new().await;
    
    // Connect first client
    let client1 = JrowClient::connect(&server.url()).await.unwrap();
    assert!(client1.is_connected());
    
    // Connect second client (should work independently)
    let client2 = JrowClient::connect(&server.url()).await.unwrap();
    assert!(client2.is_connected());
    
    // Both should be connected
    assert!(client1.is_connected());
    assert!(client2.is_connected());
    
    // Disconnect one shouldn't affect the other
    client1.disconnect().await;
    assert!(!client1.is_connected());
    assert!(client2.is_connected());
    
    client2.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_client_connection_state_transitions() {
    let server = MockWsServer::new().await;
    
    // Initial state: not connected
    let client = JrowClient::builder(&server.url())
        .without_reconnect()
        .connect()
        .await
        .unwrap();
    
    // After successful connection: connected
    assert!(client.is_connected());
    
    // After disconnect: disconnected
    client.disconnect().await;
    assert!(!client.is_connected());
    
    server.shutdown().await;
}

#[tokio::test]
async fn test_client_url_validation() {
    // Test various invalid URLs
    let invalid_urls = vec![
        "",
        "not-a-url",
        "http://wrong-protocol.com",
        "ws://",
        "://no-scheme",
    ];
    
    for url in invalid_urls {
        let result = JrowClient::connect(url).await;
        assert!(result.is_err(), "Should fail for URL: {}", url);
    }
}

#[tokio::test]
async fn test_client_connection_timeout() {
    // Test connection timeout to a non-responsive server
    // Use a routable but unreachable address
    let result = tokio::time::timeout(
        tokio::time::Duration::from_secs(2),
        JrowClient::connect("ws://192.0.2.1:8080") // TEST-NET-1, should not route
    ).await;
    
    // Should either timeout or fail to connect
    assert!(result.is_err() || result.unwrap().is_err());
}

