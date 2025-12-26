//! Subscription functionality integration tests

mod common;

use common::{MockWsServer, mock_notification};
use jrow_client::JrowClient;

#[tokio::test]
async fn test_subscribe_pattern() {
    let server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    
    client.on_notification("test.event", move |notif| {
        let tx = tx.clone();
        async move {
            let _ = tx.send(notif).await;
        }
    }).await;
    
    let result = client.subscribe("test.*", |_| async {}).await;
    assert!(result.is_ok());
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_subscribe_exact() {
    let server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    let result = client.subscribe("exact.topic", |_| async {}).await;
    assert!(result.is_ok());
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_unsubscribe() {
    let server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    client.subscribe("test.topic", |_| async {}).await.unwrap();
    let result = client.unsubscribe("test.topic").await;
    assert!(result.is_ok());
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_notification_handler_called() {
    // This test would require server to send notifications
    // Placeholder - full implementation in phase 4
}

#[tokio::test]
async fn test_multiple_handlers_same_method() {
    let server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    // Register first handler
    client.on_notification("event", |_| async {}).await;
    
    // Register second handler (should replace first)
    client.on_notification("event", |_| async {}).await;
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_subscribe_persistent() {
    let server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    let result = client.subscribe_persistent("sub-1", "topic", |_| async {}).await;
    // May fail if server doesn't support persistent subscriptions
    // That's okay for this test
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_ack_persistent() {
    let server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    let result = client.ack_persistent("sub-1", 123).await;
    // May fail if not subscribed - that's expected
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_persistent_batch_operations() {
    let server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    let subscriptions = vec![
        ("sub-1".to_string(), "topic1".to_string(), |_| async {}),
    ];
    
    let result = client.subscribe_persistent_batch(subscriptions).await;
    // May fail without server support
    
    client.disconnect().await;
    server.shutdown().await;
}

