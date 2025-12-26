//! Example demonstrating batch operations for persistent subscriptions
//!
//! This example shows how to:
//! - Subscribe to multiple persistent subscriptions at once
//! - Process messages from multiple subscriptions
//! - Batch acknowledge multiple messages
//! - Batch unsubscribe from multiple subscriptions

use jrow_client::JrowClient;
use jrow_server::{JrowServer, RetentionPolicy};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start server with persistent storage
    let server = JrowServer::builder()
        .bind_str("127.0.0.1:9004")
        .unwrap()
        .with_persistent_storage("./data/persistent_batch_example.db")
        .register_topic(
            "orders",
            RetentionPolicy {
                max_age: Some(Duration::from_secs(3600)), // 1 hour
                max_count: Some(1000),
                max_bytes: Some(10 * 1024 * 1024), // 10MB
            },
        )
        .register_topic(
            "payments",
            RetentionPolicy {
                max_age: Some(Duration::from_secs(3600)),
                max_count: Some(1000),
                max_bytes: Some(10 * 1024 * 1024),
            },
        )
        .register_topic(
            "notifications",
            RetentionPolicy {
                max_age: Some(Duration::from_secs(3600)),
                max_count: Some(1000),
                max_bytes: Some(10 * 1024 * 1024),
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

    println!("=== Batch Persistent Subscriptions Demo ===\n");

    // Connect client
    let client = JrowClient::connect("ws://127.0.0.1:9004").await?;

    // Track processed messages for each subscription
    let orders_count = Arc::new(Mutex::new(0));
    let payments_count = Arc::new(Mutex::new(0));
    let notifications_count = Arc::new(Mutex::new(0));

    // Track messages to acknowledge in batch
    let pending_acks = Arc::new(Mutex::new(Vec::new()));

    // === PART 1: Batch Subscribe ===
    println!("--- Part 1: Batch Subscribe to Multiple Persistent Subscriptions ---\n");

    // Create a unified handler that routes messages based on subscription context
    // We'll subscribe to each individually for simplicity in this example
    println!("Subscribing to 3 persistent subscriptions...");
    
    let orders_count_clone = Arc::clone(&orders_count);
    let pending_acks_clone = Arc::clone(&pending_acks);
    
    let orders_seq = client
        .subscribe_persistent("order-processor", "orders", move |msg| {
            let count = Arc::clone(&orders_count_clone);
            let acks = Arc::clone(&pending_acks_clone);
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        println!("📦 Order (seq {}): {}", seq_id, data);
                        let mut c = count.lock().await;
                        *c += 1;
                        
                        // Collect for batch ack
                        acks.lock().await.push(("order-processor".to_string(), seq_id));
                    }
                }
            }
        })
        .await?;

    let payments_count_clone = Arc::clone(&payments_count);
    let pending_acks_clone2 = Arc::clone(&pending_acks);
    
    let payments_seq = client
        .subscribe_persistent("payment-processor", "payments", move |msg| {
            let count = Arc::clone(&payments_count_clone);
            let acks = Arc::clone(&pending_acks_clone2);
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        println!("💳 Payment (seq {}): {}", seq_id, data);
                        let mut c = count.lock().await;
                        *c += 1;
                        
                        // Collect for batch ack
                        acks.lock().await.push(("payment-processor".to_string(), seq_id));
                    }
                }
            }
        })
        .await?;

    let notifications_count_clone = Arc::clone(&notifications_count);
    let pending_acks_clone3 = Arc::clone(&pending_acks);
    
    let notifications_seq = client
        .subscribe_persistent("notification-processor", "notifications", move |msg| {
            let count = Arc::clone(&notifications_count_clone);
            let acks = Arc::clone(&pending_acks_clone3);
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        println!("🔔 Notification (seq {}): {}", seq_id, data);
                        let mut c = count.lock().await;
                        *c += 1;
                        
                        // Collect for batch ack
                        acks.lock().await.push(("notification-processor".to_string(), seq_id));
                    }
                }
            }
        })
        .await?;

    let resumed_seqs = vec![
        ("order-processor".to_string(), orders_seq),
        ("payment-processor".to_string(), payments_seq),
        ("notification-processor".to_string(), notifications_seq),
    ];

    println!("✓ Subscribed to {} subscriptions:", resumed_seqs.len());
    for (sub_id, seq) in &resumed_seqs {
        println!("  - {}: resumed from sequence {}", sub_id, seq);
    }
    println!();
    println!("Note: For demonstration, subscriptions were done individually.");
    println!("In production, use subscribe_persistent_batch() when all handlers have the same type.\n");

    // === PART 2: Publish Messages ===
    println!("--- Part 2: Publishing Messages to Multiple Topics ---\n");

    println!("Publishing 5 orders...");
    for i in 1..=5 {
        server
            .publish_persistent("orders", serde_json::json!({
                "order_id": i,
                "amount": i * 100,
                "status": "pending"
            }))
            .await?;
    }

    println!("Publishing 3 payments...");
    for i in 1..=3 {
        server
            .publish_persistent("payments", serde_json::json!({
                "payment_id": i,
                "amount": i * 50,
                "method": "credit_card"
            }))
            .await?;
    }

    println!("Publishing 4 notifications...");
    for i in 1..=4 {
        server
            .publish_persistent("notifications", serde_json::json!({
                "notification_id": i,
                "message": format!("Alert {}", i),
                "priority": "high"
            }))
            .await?;
    }

    println!("✓ Published 12 messages total\n");

    // Wait for processing
    println!("Processing messages...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    let orders = *orders_count.lock().await;
    let payments = *payments_count.lock().await;
    let notifications = *notifications_count.lock().await;
    println!("✓ Processed: {} orders, {} payments, {} notifications\n", orders, payments, notifications);

    // === PART 3: Batch Acknowledge ===
    println!("--- Part 3: Batch Acknowledge Messages ---\n");

    let acks_to_send = pending_acks.lock().await.clone();
    println!("Acknowledging {} messages in batch...", acks_to_send.len());
    
    let ack_results = client.ack_persistent_batch_await(acks_to_send).await?;
    
    let successful = ack_results.iter().filter(|(_, _, success)| *success).count();
    println!("✓ Successfully acknowledged {}/{} messages", successful, ack_results.len());
    
    // Clear pending acks
    pending_acks.lock().await.clear();
    println!();

    // === PART 4: Demonstrate Resume After Unsubscribe ===
    println!("--- Part 4: Testing Resume After Batch Unsubscribe ---\n");

    // Unsubscribe from all subscriptions in batch
    println!("Unsubscribing from all subscriptions in batch...");
    client
        .unsubscribe_persistent_batch(vec![
            "order-processor".to_string(),
            "payment-processor".to_string(),
            "notification-processor".to_string(),
        ])
        .await?;
    println!("✓ Unsubscribed from all subscriptions\n");

    // Publish more messages while unsubscribed
    println!("Publishing 3 more orders while unsubscribed...");
    for i in 6..=8 {
        server
            .publish_persistent("orders", serde_json::json!({
                "order_id": i,
                "amount": i * 100,
                "status": "pending"
            }))
            .await?;
    }
    println!("✓ Published 3 orders\n");

    // Reset counters
    *orders_count.lock().await = 0;
    *payments_count.lock().await = 0;
    *notifications_count.lock().await = 0;

    // Resubscribe - should receive undelivered messages
    println!("Resubscribing...");
    
    let client_clone = client.clone();
    let orders_count_clone = Arc::clone(&orders_count);

    let resumed_seq2 = client
        .subscribe_persistent("order-processor", "orders", move |msg| {
            let client = client_clone.clone();
            let count = Arc::clone(&orders_count_clone);
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        println!("📦 Resumed Order (seq {}): {}", seq_id, data);
                        let mut c = count.lock().await;
                        *c += 1;
                        
                        // Acknowledge immediately for this demo
                        client.ack_persistent("order-processor", seq_id);
                    }
                }
            }
        })
        .await?;

    println!("✓ Resubscribed: order-processor resumed from sequence {}", resumed_seq2);
    println!();

    // Wait for resumed messages
    tokio::time::sleep(Duration::from_secs(1)).await;

    let resumed_orders = *orders_count.lock().await;
    println!("✓ Received {} undelivered orders after resume\n", resumed_orders);

    // === Summary ===
    println!("--- Summary ---\n");
    println!("✓ Demonstrated multiple persistent subscriptions");
    println!("✓ Demonstrated batch acknowledgment of messages (ack_persistent_batch_await)");
    println!("✓ Demonstrated batch unsubscribe (unsubscribe_persistent_batch)");
    println!("✓ Demonstrated resume capability after batch operations");
    println!("\nBatch Operations Available:");
    println!("  - subscribe_persistent_batch() - Subscribe to multiple subscriptions at once");
    println!("  - ack_persistent_batch() - Acknowledge multiple messages (fire-and-forget)");
    println!("  - ack_persistent_batch_await() - Acknowledge multiple messages (await results)");
    println!("  - unsubscribe_persistent_batch() - Unsubscribe from multiple subscriptions");
    println!("\nBenefits of batching:");
    println!("  - Reduced network round-trips");
    println!("  - Lower latency for bulk operations");
    println!("  - More efficient resource utilization");
    println!("\nDatabase persisted at: ./data/persistent_batch_example.db");

    // Exit cleanly
    std::process::exit(0);
}

