//! Connection lifecycle integration tests for jrow-server

use jrow_server::{JrowServer, from_fn};
use jrow_client::JrowClient;
use std::time::Duration;

#[tokio::test]
async fn test_connection_upgrade_success() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .handler("ping", from_fn(|_| async {
            Ok(serde_json::json!({"pong": true}))
        }))
        .build()
        .await
        .unwrap();
    
    let server_addr = server.local_addr().unwrap();
    
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", server_addr);
    let client = JrowClient::connect(&url).await;
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_connection_message_routing() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .handler("echo", from_fn(|params| async move {
            Ok(params.unwrap_or(serde_json::json!(null)))
        }))
        .build()
        .await
        .unwrap();
    
    let server_addr = server.local_addr().unwrap();
    
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", server_addr);
    let client = JrowClient::connect(&url).await.unwrap();
    
    let result: serde_json::Value = client.request("echo", Some(serde_json::json!({"test": "data"}))).await.unwrap();
    assert_eq!(result["test"], "data");
}

#[tokio::test]
async fn test_connection_error_response() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .build()
        .await
        .unwrap();
    
    let server_addr = server.local_addr().unwrap();
    
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", server_addr);
    let client = JrowClient::connect(&url).await.unwrap();
    
    // Call non-existent method
    let result: Result<serde_json::Value, _> = client.request("nonexistent", None::<serde_json::Value>).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_connection_batch_processing() {
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
    
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Batch request test would require batch support in client
    // Placeholder for now
}

