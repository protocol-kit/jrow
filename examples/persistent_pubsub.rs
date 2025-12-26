//! Example demonstrating persistent subscriptions with exactly-once delivery

use jrow_client::JrowClient;
use jrow_server::{JrowServer, RetentionPolicy};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start server with persistent storage
    let server = JrowServer::builder()
        .bind_str("127.0.0.1:9003")
        .unwrap()
        .with_persistent_storage("./data/persistent_example.db")
        .register_topic(
            "orders",
            RetentionPolicy {
                max_age: Some(Duration::from_secs(3600)), // 1 hour
                max_count: Some(1000),
                max_bytes: Some(10 * 1024 * 1024), // 10MB
            },
        )
        .subscription_timeout(Duration::from_secs(300)) // 5 minutes
        .retention_interval(Duration::from_secs(30))
        .build()
        .await?;

    // Spawn server in background
    let server = Arc::new(server);
    let server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        if let Err(e) = server_clone.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = JrowClient::connect("ws://127.0.0.1:9003").await?;

    // Track processed messages
    let processed_count = Arc::new(Mutex::new(0));
    let processed_clone = Arc::clone(&processed_count);

    // Subscribe with persistent tracking
    let client_clone = client.clone();
    let resumed_seq = client
        .subscribe_persistent("order-processor-1", "orders", move |msg| {
            let client = client_clone.clone();
            let processed = Arc::clone(&processed_clone);
            async move {
                // Extract sequence_id and data from the persistent message format
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        println!("Processing order (seq {}): {}", seq_id, data);

                        // Simulate processing
                        tokio::time::sleep(Duration::from_millis(10)).await;

                        // Acknowledge after successful processing
                        // Note: ack_persistent spawns internally, so it won't block the handler
                        client.ack_persistent("order-processor-1", seq_id);
                        
                        let mut count = processed.lock().await;
                        *count += 1;
                        println!("Acknowledged message {} (total: {})", seq_id, *count);
                    }
                }
            }
        })
        .await?;

    println!("Subscribed! Resumed from sequence: {}", resumed_seq);

    // Publish some messages
    println!("\nPublishing 10 messages...");
    for i in 1..=10 {
        let seq = server
            .publish_persistent("orders", serde_json::json!({
                "order_id": i,
                "amount": i * 100,
                "status": "pending"
            }))
            .await?;
        println!("Published message with sequence ID: {}", seq);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Wait for processing
    println!("\nWaiting for messages to be processed...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let final_count = *processed_count.lock().await;
    println!("\nProcessed {} messages", final_count);

    // Demonstrate persistence: unsubscribe and resubscribe
    println!("\n--- Testing Resume Capability ---");
    client.unsubscribe_persistent("order-processor-1").await?;
    println!("Unsubscribed");

    // Publish more messages while unsubscribed
    println!("Publishing 5 more messages while unsubscribed...");
    for i in 11..=15 {
        let seq = server
            .publish_persistent("orders", serde_json::json!({
                "order_id": i,
                "amount": i * 100,
                "status": "pending"
            }))
            .await?;
        println!("Published message with sequence ID: {}", seq);
    }

    // Resubscribe - should receive unacknowledged messages
    println!("\nResubscribing...");
    let processed_count2 = Arc::new(Mutex::new(0));
    let processed_clone2 = Arc::clone(&processed_count2);
    let client_clone2 = client.clone();

    let resumed_seq2 = client
        .subscribe_persistent("order-processor-1", "orders", move |msg| {
            let client = client_clone2.clone();
            let processed = Arc::clone(&processed_clone2);
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        println!("Processing resumed order (seq {}): {}", seq_id, data);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        
                        // Acknowledge the message
                        client.ack_persistent("order-processor-1", seq_id);
                        
                        let mut count = processed.lock().await;
                        *count += 1;
                        println!("Acknowledged message {}", seq_id);
                    }
                }
            }
        })
        .await?;

    println!("Resumed from sequence: {}", resumed_seq2);
    println!("Should receive 5 undelivered messages...");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let final_count2 = *processed_count2.lock().await;
    println!("\nProcessed {} messages after resume", final_count2);

    // Demonstrate reconnection with automatic subscription resume
    println!("\n--- Testing Reconnection ---");
    
    // For immediate reconnection with the same subscription ID, it's best to 
    // explicitly unsubscribe first. The server DOES auto-cleanup on disconnect,
    // but connection close is asynchronous and may take time.
    println!("Unsubscribing before disconnect (for immediate reconnection)...");
    client.unsubscribe_persistent("order-processor-1").await?;
    
    println!("Disconnecting client...");
    drop(client);
    
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Publish messages while disconnected
    println!("Publishing 3 messages while disconnected...");
    for i in 19..=21 {
        let seq = server
            .publish_persistent("orders", serde_json::json!({
                "order_id": i,
                "amount": i * 100,
                "status": "pending"
            }))
            .await?;
        println!("Published message with sequence ID: {}", seq);
    }

    // Reconnect with a new client
    println!("\nReconnecting with new client...");
    let client = JrowClient::connect("ws://127.0.0.1:9003").await?;

    let processed_count3 = Arc::new(Mutex::new(0));
    let processed_clone3 = Arc::clone(&processed_count3);
    let client_clone3 = client.clone();

    println!("Resubscribing after reconnection...");
    let resumed_seq3 = client
        .subscribe_persistent("order-processor-1", "orders", move |msg| {
            let client = client_clone3.clone();
            let processed = Arc::clone(&processed_clone3);
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        println!("Processing reconnected order (seq {}): {}", seq_id, data);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        
                        client.ack_persistent("order-processor-1", seq_id);
                        
                        let mut count = processed.lock().await;
                        *count += 1;
                        println!("Acknowledged message {}", seq_id);
                    }
                }
            }
        })
        .await?;

    println!("Resumed from sequence: {}", resumed_seq3);
    println!("Should receive 3 messages that were published while disconnected...");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let final_count3 = *processed_count3.lock().await;
    println!("\nProcessed {} messages after reconnection", final_count3);

    println!("\n✓ Persistent subscriptions example completed!");
    println!("✓ Demonstrated: persistence, resume, and reconnection");
    println!("Database persisted at: ./data/persistent_example.db");

    // Exit cleanly (the server task will be cancelled when main exits)
    std::process::exit(0);
}

