//! WebSocket protocol lifecycle edge case tests

use jrow_server::{JrowServer, from_fn};
use jrow_client::JrowClient;
use std::time::Duration;

#[tokio::test]
async fn test_text_message_handling() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .handler("test", from_fn(|_| async {
            Ok(serde_json::json!({"result": "ok"}))
        }))
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
    
    let result: serde_json::Value = client.request("test", None::<serde_json::Value>).await.unwrap();
    assert_eq!(result["result"], "ok");
    
    client.disconnect().await;
    server_handle.abort();
}

#[tokio::test]
async fn test_connection_close_handshake() {
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
    
    // Graceful disconnect should complete close handshake
    client.disconnect().await;
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    server_handle.abort();
}

#[tokio::test]
async fn test_multiple_sequential_connections() {
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
    
    // Connect, disconnect, connect again
    for _ in 0..3 {
        let client = JrowClient::connect(&url).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        client.disconnect().await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    server_handle.abort();
}

