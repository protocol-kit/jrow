//! End-to-end pub/sub integration tests

use jrow_server::{JrowServer, from_fn};
use jrow_client::JrowClient;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_pubsub_exact_match() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    let server_addr = server.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", server_addr);
    let client = JrowClient::connect(&url).await.unwrap();
    
    let (tx, mut rx) = mpsc::channel(10);
    
    client.on_notification("test.event", move |notif| {
        let tx = tx.clone();
        async move {
            let _ = tx.send(notif).await;
        }
    }).await;
    
    client.subscribe("test.event", |_| async {}).await.unwrap();
    
    // Give subscription time to register
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    client.disconnect().await;
    server_handle.abort();
}

#[tokio::test]
async fn test_pubsub_single_wildcard() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    let server_addr = server.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", server_addr);
    let client = JrowClient::connect(&url).await.unwrap();
    
    client.subscribe("events.*", |_| async {}).await.unwrap();
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    client.disconnect().await;
    server_handle.abort();
}

#[tokio::test]
async fn test_pubsub_multi_wildcard() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    let server_addr = server.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", server_addr);
    let client = JrowClient::connect(&url).await.unwrap();
    
    client.subscribe("events.>", |_| async {}).await.unwrap();
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    client.disconnect().await;
    server_handle.abort();
}

#[tokio::test]
async fn test_pubsub_multiple_subscribers() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    let server_addr = server.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", server_addr);
    
    let client1 = JrowClient::connect(&url).await.unwrap();
    let client2 = JrowClient::connect(&url).await.unwrap();
    
    client1.subscribe("topic", |_| async {}).await.unwrap();
    client2.subscribe("topic", |_| async {}).await.unwrap();
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    client1.disconnect().await;
    client2.disconnect().await;
    server_handle.abort();
}

#[tokio::test]
async fn test_pubsub_unsubscribe_stops_notifications() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    let server_addr = server.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", server_addr);
    let client = JrowClient::connect(&url).await.unwrap();
    
    client.subscribe("topic", |_| async {}).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    client.unsubscribe("topic").await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    client.disconnect().await;
    server_handle.abort();
}

