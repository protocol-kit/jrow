//! Integration tests for persistent subscriptions

use jrow_client::JrowClient;
use jrow_server::{JrowServer, RetentionPolicy};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_persistent_publish_and_subscribe() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_basic.db");

    // Start server
    let server = JrowServer::builder()
        .bind_str("127.0.0.1:0")
        .unwrap()
        .with_persistent_storage(&db_path)
        .register_topic("test_topic", RetentionPolicy::unlimited())
        .build()
        .await
        .unwrap();

    let addr = server.local_addr().unwrap();

    // Spawn server
    let server = Arc::new(server);
    let server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        server_clone.run().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = JrowClient::connect(&format!("ws://{}", addr))
        .await
        .unwrap();

    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = Arc::clone(&received);
    let client_clone = client.clone();

    // Subscribe
    client
        .subscribe_persistent("test_sub_1", "test_topic", move |msg| {
            let received = Arc::clone(&received_clone);
            let client = client_clone.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let Some(seq_id) = obj.get("sequence_id").and_then(|v| v.as_u64()) {
                        received.lock().await.push(seq_id);
                        client.ack_persistent("test_sub_1", seq_id);
                    }
                }
            }
        })
        .await
        .unwrap();

    // Give subscription time to register
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish messages
    for i in 1..=5 {
        server
            .publish_persistent("test_topic", serde_json::json!({ "value": i }))
            .await
            .unwrap();
    }

    // Wait for delivery and acks
    tokio::time::sleep(Duration::from_millis(500)).await;

    let received_list = received.lock().await;
    assert_eq!(received_list.len(), 5);
    assert_eq!(*received_list, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_persistent_resume_after_unsubscribe() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_resume.db");

    let server = JrowServer::builder()
        .bind_str("127.0.0.1:0")
        .unwrap()
        .with_persistent_storage(&db_path)
        .register_topic("resume_topic", RetentionPolicy::unlimited())
        .build()
        .await
        .unwrap();

    let addr = server.local_addr().unwrap();
    let server = Arc::new(server);
    let server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        server_clone.run().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = JrowClient::connect(&format!("ws://{}", addr))
        .await
        .unwrap();

    // First subscription - process some messages
    let received1 = Arc::new(Mutex::new(Vec::new()));
    let received1_clone = Arc::clone(&received1);
    let client_clone1 = client.clone();

    client
        .subscribe_persistent("resume_sub", "resume_topic", move |msg| {
            let received = Arc::clone(&received1_clone);
            let client = client_clone1.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let Some(seq_id) = obj.get("sequence_id").and_then(|v| v.as_u64()) {
                        received.lock().await.push(seq_id);
                        // Only ack first 3 messages
                        if seq_id <= 3 {
                            client.ack_persistent("resume_sub", seq_id);
                        }
                    }
                }
            }
        })
        .await
        .unwrap();

    // Publish 5 messages
    for i in 1..=5 {
        server
            .publish_persistent("resume_topic", serde_json::json!({ "value": i }))
            .await
            .unwrap();
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(received1.lock().await.len(), 5);

    // Unsubscribe
    client.unsubscribe_persistent("resume_sub").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Resubscribe - should only get messages after seq 3
    let received2 = Arc::new(Mutex::new(Vec::new()));
    let received2_clone = Arc::clone(&received2);
    let client_clone2 = client.clone();

    let resumed_seq = client
        .subscribe_persistent("resume_sub", "resume_topic", move |msg| {
            let received = Arc::clone(&received2_clone);
            let client = client_clone2.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let Some(seq_id) = obj.get("sequence_id").and_then(|v| v.as_u64()) {
                        received.lock().await.push(seq_id);
                        client.ack_persistent("resume_sub", seq_id);
                    }
                }
            }
        })
        .await
        .unwrap();

    assert_eq!(resumed_seq, 3); // Last ack'd was 3
    tokio::time::sleep(Duration::from_millis(500)).await;

    let list2 = received2.lock().await;
    assert_eq!(*list2, vec![4, 5]); // Should only receive unacknowledged messages
}

#[tokio::test]
async fn test_retention_by_count() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_retention.db");

    let server = JrowServer::builder()
        .bind_str("127.0.0.1:0")
        .unwrap()
        .with_persistent_storage(&db_path)
        .register_topic("retention_topic", RetentionPolicy::by_count(3))
        .retention_interval(Duration::from_millis(100))
        .build()
        .await
        .unwrap();

    let server = Arc::new(server);
    let addr = server.local_addr().unwrap();
    let server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        server_clone.run().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish 5 messages
    for i in 1..=5 {
        server
            .publish_persistent("retention_topic", serde_json::json!({ "value": i }))
            .await
            .unwrap();
    }

    // Wait for retention task to run
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Check that only 3 messages remain
    let storage = server.persistent_storage().unwrap();
    let messages = storage.get_messages_since("retention_topic", 0).await.unwrap();
    assert!(messages.len() <= 3);
}

#[tokio::test]
async fn test_exclusive_subscription() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_exclusive.db");

    let server = JrowServer::builder()
        .bind_str("127.0.0.1:0")
        .unwrap()
        .with_persistent_storage(&db_path)
        .register_topic("exclusive_topic", RetentionPolicy::unlimited())
        .build()
        .await
        .unwrap();

    let addr = server.local_addr().unwrap();
    let server = Arc::new(server);
    let server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        server_clone.run().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect two clients
    let client1 = JrowClient::connect(&format!("ws://{}", addr))
        .await
        .unwrap();
    let client2 = JrowClient::connect(&format!("ws://{}", addr))
        .await
        .unwrap();

    // First client subscribes
    client1
        .subscribe_persistent("exclusive_sub", "exclusive_topic", |_msg| async {})
        .await
        .unwrap();

    // Second client tries to use same subscription ID - should fail
    let result = client2
        .subscribe_persistent("exclusive_sub", "exclusive_topic", |_msg| async {})
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_multiple_topics() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_multi_topic.db");

    let server = JrowServer::builder()
        .bind_str("127.0.0.1:0")
        .unwrap()
        .with_persistent_storage(&db_path)
        .register_topic("topic_a", RetentionPolicy::unlimited())
        .register_topic("topic_b", RetentionPolicy::unlimited())
        .build()
        .await
        .unwrap();

    let addr = server.local_addr().unwrap();
    let server = Arc::new(server);
    let server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        server_clone.run().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = JrowClient::connect(&format!("ws://{}", addr))
        .await
        .unwrap();

    let received_a = Arc::new(Mutex::new(Vec::new()));
    let received_b = Arc::new(Mutex::new(Vec::new()));

    let received_a_clone = Arc::clone(&received_a);
    let client_a = client.clone();
    client
        .subscribe_persistent("sub_a", "topic_a", move |msg| {
            let received = Arc::clone(&received_a_clone);
            let client = client_a.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let Some(seq_id) = obj.get("sequence_id").and_then(|v| v.as_u64()) {
                        received.lock().await.push(seq_id);
                        client.ack_persistent("sub_a", seq_id);
                    }
                }
            }
        })
        .await
        .unwrap();

    let received_b_clone = Arc::clone(&received_b);
    let client_b = client.clone();
    client
        .subscribe_persistent("sub_b", "topic_b", move |msg| {
            let received = Arc::clone(&received_b_clone);
            let client = client_b.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let Some(seq_id) = obj.get("sequence_id").and_then(|v| v.as_u64()) {
                        received.lock().await.push(seq_id);
                        client.ack_persistent("sub_b", seq_id);
                    }
                }
            }
        })
        .await
        .unwrap();

    // Publish to both topics
    for i in 1..=3 {
        server
            .publish_persistent("topic_a", serde_json::json!({ "topic": "a", "value": i }))
            .await
            .unwrap();
        server
            .publish_persistent("topic_b", serde_json::json!({ "topic": "b", "value": i }))
            .await
            .unwrap();
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    assert_eq!(received_a.lock().await.len(), 3);
    assert_eq!(received_b.lock().await.len(), 3);
}

