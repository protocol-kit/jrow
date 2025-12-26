//! Full client-server integration tests

use jrow_server::{JrowServer, from_fn, from_typed_fn};
use jrow_client::JrowClient;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct AddParams {
    a: i32,
    b: i32,
}

#[tokio::test]
async fn test_full_rpc_roundtrip() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = JrowServer::builder()
        .bind(addr)
        .handler("add", from_typed_fn(|p: AddParams| async move {
            Ok(p.a + p.b)
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
    
    let result: i32 = client.request_typed("add", AddParams { a: 5, b: 3 }).await.unwrap();
    assert_eq!(result, 8);
    
    client.disconnect().await;
    server_handle.abort();
}

#[tokio::test]
async fn test_error_propagation() {
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
    
    let result: Result<serde_json::Value, _> = client.request("nonexistent", None::<serde_json::Value>).await;
    assert!(result.is_err());
    
    client.disconnect().await;
    server_handle.abort();
}

#[tokio::test]
async fn test_multiple_clients_same_server() {
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
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", server_addr);
    
    let client1 = JrowClient::connect(&url).await.unwrap();
    let client2 = JrowClient::connect(&url).await.unwrap();
    
    let result1: serde_json::Value = client1.request("ping", None::<serde_json::Value>).await.unwrap();
    let result2: serde_json::Value = client2.request("ping", None::<serde_json::Value>).await.unwrap();
    
    assert_eq!(result1["pong"], true);
    assert_eq!(result2["pong"], true);
    
    client1.disconnect().await;
    client2.disconnect().await;
    server_handle.abort();
}

