//! Server example for NATS-style pattern matching with persistent subscriptions
//!
//! This server publishes messages to various topics every few seconds.
//! Run the client in another terminal to see pattern matching in action.
//!
//! Usage:
//!   Terminal 1: cargo run --example persistent_pattern_server
//!   Terminal 2: cargo run --example persistent_pattern_client
//!
//! Try stopping and restarting the client to see automatic resume!

use jrow_server::{JrowServer, RetentionPolicy};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== NATS Pattern Matching - Server ===\n");
    println!("Starting server on 127.0.0.1:9007...");

    // Build server with persistent storage
    let server = JrowServer::builder()
        .bind_str("127.0.0.1:9007")?
        .with_persistent_storage("./data/pattern_server.db")
        .register_topic(
            "orders",
            RetentionPolicy {
                max_age: Some(Duration::from_secs(3600)),
                max_count: Some(1000),
                max_bytes: Some(10 * 1024 * 1024),
            },
        )
        .register_topic(
            "events",
            RetentionPolicy {
                max_age: Some(Duration::from_secs(3600)),
                max_count: Some(1000),
                max_bytes: Some(10 * 1024 * 1024),
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
        .subscription_timeout(Duration::from_secs(300))
        .retention_interval(Duration::from_secs(60))
        .build()
        .await?;

    let server = Arc::new(server);
    let server_clone = Arc::clone(&server);

    // Spawn server task
    tokio::spawn(async move {
        if let Err(e) = server_clone.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("✓ Server running\n");
    println!("Publishing messages to various topics...");
    println!("Patterns to try:");
    println!("  - 'orders.*' matches: orders.new, orders.shipped, orders.cancelled");
    println!("  - 'orders.>' matches: orders.new, orders.new.express, orders.shipped.domestic");
    println!("  - 'events.user.>' matches: events.user.login, events.user.login.success");
    println!("  - '*.*.success' matches: events.user.success, payments.card.success\n");

    let mut counter = 0u64;

    loop {
        counter += 1;

        // Publish order events
        let order_id = 1000 + counter;
        
        // orders.new
        let seq = server.publish_persistent(
            "orders.new",
            serde_json::json!({
                "order_id": order_id,
                "status": "new",
                "amount": 99.99,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        ).await?;
        println!("[orders.new] seq={} order_id={}", seq, order_id);

        tokio::time::sleep(Duration::from_secs(2)).await;

        // orders.shipped
        let seq = server.publish_persistent(
            "orders.shipped",
            serde_json::json!({
                "order_id": order_id,
                "status": "shipped",
                "tracking": format!("TRK{}", order_id),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        ).await?;
        println!("[orders.shipped] seq={} order_id={}", seq, order_id);

        tokio::time::sleep(Duration::from_secs(2)).await;

        // orders.new.express (two tokens after orders)
        let seq = server.publish_persistent(
            "orders.new.express",
            serde_json::json!({
                "order_id": order_id + 1000,
                "status": "new",
                "type": "express",
                "amount": 149.99,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        ).await?;
        println!("[orders.new.express] seq={} order_id={}", seq, order_id + 1000);

        tokio::time::sleep(Duration::from_secs(2)).await;

        // events.user.login
        let seq = server.publish_persistent(
            "events.user.login",
            serde_json::json!({
                "event": "user_login",
                "user_id": 100 + counter,
                "ip": "192.168.1.1",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        ).await?;
        println!("[events.user.login] seq={} user_id={}", seq, 100 + counter);

        tokio::time::sleep(Duration::from_secs(2)).await;

        // events.user.login.success (deeply nested)
        let seq = server.publish_persistent(
            "events.user.login.success",
            serde_json::json!({
                "event": "login_success",
                "user_id": 100 + counter,
                "session_id": format!("sess_{}", counter),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        ).await?;
        println!("[events.user.login.success] seq={} user_id={}", seq, 100 + counter);

        tokio::time::sleep(Duration::from_secs(2)).await;

        // payments.card.success
        let seq = server.publish_persistent(
            "payments.card.success",
            serde_json::json!({
                "payment_id": format!("pay_{}", counter),
                "method": "card",
                "status": "success",
                "amount": 99.99,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        ).await?;
        println!("[payments.card.success] seq={} payment_id=pay_{}", seq, counter);

        tokio::time::sleep(Duration::from_secs(2)).await;

        // orders.cancelled
        if counter % 3 == 0 {
            let seq = server.publish_persistent(
                "orders.cancelled",
                serde_json::json!({
                    "order_id": order_id - 1,
                    "status": "cancelled",
                    "reason": "customer_request",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })
            ).await?;
            println!("[orders.cancelled] seq={} order_id={}", seq, order_id - 1);
        }

        println!("---");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}



