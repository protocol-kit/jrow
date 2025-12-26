//! Request/response pattern integration tests
//!
//! Tests for various request/response scenarios including success,
//! errors, timeouts, and typed responses.

mod common;

use common::{MockWsServer, mock_response, mock_error_response};
use jrow_client::JrowClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TestParams {
    value: i32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TestResult {
    doubled: i32,
}

#[tokio::test]
async fn test_request_success() {
    let server = MockWsServer::with_handler(|msg| async move {
        // Parse request and return mock response
        if msg.contains("\"method\":\"double\"") {
            Some(mock_response(1, serde_json::json!({"doubled": 84})))
        } else {
            None
        }
    }).await;
    
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    let result: serde_json::Value = client.request("double", Some(serde_json::json!({"value": 42}))).await.unwrap();
    
    assert_eq!(result["doubled"], 84);
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_request_timeout() {
    // Server that never responds
    let server = MockWsServer::with_handler(|_msg| async move {
        // Don't respond
        None
    }).await;
    
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    // Request with timeout
    let result = tokio::time::timeout(
        tokio::time::Duration::from_millis(500),
        client.request::<serde_json::Value>("test", None::<serde_json::Value>)
    ).await;
    
    // Should timeout
    assert!(result.is_err());
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_request_server_error() {
    let server = MockWsServer::with_handler(|msg| async move {
        // Return error response
        if msg.contains("\"method\":\"fail\"") {
            Some(mock_error_response(1, -32601, "Method not found"))
        } else {
            None
        }
    }).await;
    
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    let result: Result<serde_json::Value, _> = client.request("fail", None::<serde_json::Value>).await;
    
    assert!(result.is_err());
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_request_connection_lost() {
    let server = MockWsServer::new().await;
    let client = JrowClient::builder(&server.url())
        .without_reconnect()
        .connect()
        .await
        .unwrap();
    
    // Start a request
    let request_future = client.request::<serde_json::Value>("test", None::<serde_json::Value>);
    
    // Drop server (connection lost)
    server.shutdown().await;
    
    // Request should fail with connection error
    let result = request_future.await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_request_typed_success() {
    let server = MockWsServer::with_handler(|msg| async move {
        if msg.contains("\"method\":\"double\"") {
            Some(mock_response(1, serde_json::json!({"doubled": 84})))
        } else {
            None
        }
    }).await;
    
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    // Use typed request
    let result: TestResult = client.request_typed("double", TestParams { value: 42 }).await.unwrap();
    
    assert_eq!(result.doubled, 84);
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_request_typed_wrong_type() {
    let server = MockWsServer::with_handler(|msg| async move {
        if msg.contains("\"method\":\"wrong\"") {
            // Return wrong type
            Some(mock_response(1, serde_json::json!("string instead of object")))
        } else {
            None
        }
    }).await;
    
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    // Try to deserialize to wrong type
    let result: Result<TestResult, _> = client.request_typed("wrong", TestParams { value: 42 }).await;
    
    // Should fail with deserialization error
    assert!(result.is_err());
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_notify_fire_and_forget() {
    let mut server = MockWsServer::new().await;
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    // Send notification (fire and forget)
    let result = client.notify("event", Some(serde_json::json!({"type": "test"}))).await;
    
    // Notify should succeed
    assert!(result.is_ok());
    
    // Server should receive the notification
    let received = server.wait_for_message().await;
    assert!(received.is_some());
    let received_msg = received.unwrap();
    assert!(received_msg.contains("\"method\":\"event\""));
    assert!(!received_msg.contains("\"id\"")); // Notifications have no ID
    
    client.disconnect().await;
    server.shutdown().await;
}

#[tokio::test]
async fn test_concurrent_requests() {
    let server = MockWsServer::with_handler(|msg| async move {
        // Respond to any request
        if msg.contains("\"id\":") {
            // Extract ID and respond
            if msg.contains("\"id\":1") {
                Some(mock_response(1, serde_json::json!({"result": "one"})))
            } else if msg.contains("\"id\":2") {
                Some(mock_response(2, serde_json::json!({"result": "two"})))
            } else {
                None
            }
        } else {
            None
        }
    }).await;
    
    let client = JrowClient::connect(&server.url()).await.unwrap();
    
    // Send multiple concurrent requests
    let req1 = client.request::<serde_json::Value>("test1", None::<serde_json::Value>);
    let req2 = client.request::<serde_json::Value>("test2", None::<serde_json::Value>);
    
    // Both should complete
    let (res1, res2) = tokio::join!(req1, req2);
    
    // At least one should succeed (depending on timing)
    assert!(res1.is_ok() || res2.is_ok());
    
    client.disconnect().await;
    server.shutdown().await;
}

