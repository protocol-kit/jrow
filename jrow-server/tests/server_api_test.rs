//! Server public API integration tests

use jrow_server::{JrowServer, from_fn};
use std::time::Duration;

#[tokio::test]
async fn test_server_local_addr() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    let local_addr = server.local_addr();
    assert!(local_addr.is_ok());
    assert_ne!(local_addr.unwrap().port(), 0);
}

#[tokio::test]
async fn test_server_publish_to_exact_topic() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    // Publish to topic (no subscribers initially)
    let count = server.publish("test.topic", serde_json::json!({"data": "test"})).await;
    assert!(count.is_ok());
    assert_eq!(count.unwrap(), 0); // No subscribers
}

#[tokio::test]
async fn test_server_publish_batch() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    let messages = vec![
        ("topic1".to_string(), serde_json::json!({"msg": 1})),
        ("topic2".to_string(), serde_json::json!({"msg": 2})),
    ];
    
    let result = server.publish_batch(messages).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_server_publish_no_subscribers() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    // Publish should succeed even with no subscribers
    let result = server.publish("unused.topic", serde_json::json!({"test": true})).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_server_drop_cleanup() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    // Drop the server
    drop(server);
    
    // If we get here without panic, cleanup worked
    tokio::time::sleep(Duration::from_millis(100)).await;
}

