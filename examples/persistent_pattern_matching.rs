//! Example demonstrating NATS-style pattern matching for persistent subscriptions
//!
//! Shows how to use token-based patterns:
//! - `*` matches exactly one token
//! - `>` matches one or more tokens (must be at end)
//! - `.` is the token delimiter

use jrow_client::JrowClient;
use jrow_server::{JrowServer, RetentionPolicy};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== NATS-Style Pattern Matching for Persistent Subscriptions ===\n");

    // Start server with persistent storage
    let server = JrowServer::builder()
        .bind_str("127.0.0.1:9006")
        .unwrap()
        .with_persistent_storage("./data/pattern_matching_example.db")
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
        .subscription_timeout(Duration::from_secs(300))
        .retention_interval(Duration::from_secs(30))
        .build()
        .await?;

    let server = Arc::new(server);
    let server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        if let Err(e) = server_clone.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = JrowClient::connect("ws://127.0.0.1:9006").await?;

    // Track received messages
    let exact_count = Arc::new(Mutex::new(0));
    let single_wild_count = Arc::new(Mutex::new(0));
    let multi_wild_count = Arc::new(Mutex::new(0));

    println!("--- Part 1: Exact Topic Subscription ---\n");

    let exact_clone = Arc::clone(&exact_count);
    let client_clone = client.clone();
    let exact_seq = client
        .subscribe_persistent("exact-sub", "orders.new", move |msg| {
            let count = Arc::clone(&exact_clone);
            let client = client_clone.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("unknown");
                        println!("  [EXACT] {} (seq {}): {}", topic, seq_id, data);
                        *count.lock().await += 1;
                        client.ack_persistent("exact-sub", seq_id);
                    }
                }
            }
        })
        .await?;

    println!("✓ Subscribed to exact topic 'orders.new' (resumed from seq {})\n", exact_seq);

    println!("--- Part 2: Single Wildcard Pattern (*) ---\n");

    let single_clone = Arc::clone(&single_wild_count);
    let client_clone2 = client.clone();
    let single_seq = client
        .subscribe_persistent("single-wild-sub", "orders.*", move |msg| {
            let count = Arc::clone(&single_clone);
            let client = client_clone2.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("unknown");
                        println!("  [SINGLE *] {} (seq {}): {}", topic, seq_id, data);
                        *count.lock().await += 1;
                        client.ack_persistent("single-wild-sub", seq_id);
                    }
                }
            }
        })
        .await?;

    println!("✓ Subscribed to pattern 'orders.*' (resumed from seq {})", single_seq);
    println!("  Matches: orders.new, orders.shipped, orders.cancelled\n");

    println!("--- Part 3: Multi-Token Wildcard Pattern (>) ---\n");

    let multi_clone = Arc::clone(&multi_wild_count);
    let client_clone3 = client.clone();
    let multi_seq = client
        .subscribe_persistent("multi-wild-sub", "events.>", move |msg| {
            let count = Arc::clone(&multi_clone);
            let client = client_clone3.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("unknown");
                        println!("  [MULTI >] {} (seq {}): {}", topic, seq_id, data);
                        *count.lock().await += 1;
                        client.ack_persistent("multi-wild-sub", seq_id);
                    }
                }
            }
        })
        .await?;

    println!("✓ Subscribed to pattern 'events.>' (resumed from seq {})", multi_seq);
    println!("  Matches: events.user, events.user.login, events.user.login.success\n");

    println!("--- Part 4: Publishing Messages ---\n");

    // Exact match
    println!("Publishing to orders.new:");
    server.publish_persistent("orders.new", serde_json::json!({
        "order_id": 1,
        "status": "created"
    })).await?;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Matches single wildcard
    println!("Publishing to orders.shipped:");
    server.publish_persistent("orders.shipped", serde_json::json!({
        "order_id": 2,
        "status": "shipped"
    })).await?;
    tokio::time::sleep(Duration::from_millis(50)).await;

    println!("Publishing to orders.cancelled:");
    server.publish_persistent("orders.cancelled", serde_json::json!({
        "order_id": 3,
        "status": "cancelled"
    })).await?;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Matches multi wildcard
    println!("Publishing to events.user:");
    server.publish_persistent("events.user", serde_json::json!({
        "event": "user_created",
        "user_id": 123
    })).await?;
    tokio::time::sleep(Duration::from_millis(50)).await;

    println!("Publishing to events.user.login:");
    server.publish_persistent("events.user.login", serde_json::json!({
        "event": "login",
        "user_id": 123
    })).await?;
    tokio::time::sleep(Duration::from_millis(50)).await;

    println!("Publishing to events.user.login.success:");
    server.publish_persistent("events.user.login.success", serde_json::json!({
        "event": "login_success",
        "user_id": 123,
        "timestamp": "2025-01-01T00:00:00Z"
    })).await?;
    tokio::time::sleep(Duration::from_millis(50)).await;

    println!("\nWaiting for messages to be processed...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("\n--- Results ---\n");
    let exact = *exact_count.lock().await;
    let single = *single_wild_count.lock().await;
    let multi = *multi_wild_count.lock().await;

    println!("Exact subscription (orders.new): {} messages", exact);
    println!("Single wildcard (orders.*): {} messages", single);
    println!("Multi wildcard (events.>): {} messages", multi);

    println!("\n--- Pattern Matching Rules ---\n");
    println!("Pattern: orders.new (exact)");
    println!("  ✓ Matches: orders.new");
    println!("  ✗ Doesn't match: orders.shipped, orders.new.fast\n");

    println!("Pattern: orders.* (single wildcard)");
    println!("  ✓ Matches: orders.new, orders.shipped, orders.cancelled");
    println!("  ✗ Doesn't match: orders, orders.new.fast, events.new\n");

    println!("Pattern: events.> (multi wildcard)");
    println!("  ✓ Matches: events.user, events.user.login, events.user.login.success");
    println!("  ✗ Doesn't match: events, orders.user\n");

    println!("--- Pattern Syntax Summary ---\n");
    println!("• `.` (dot) separates tokens");
    println!("• `*` matches exactly ONE token");
    println!("• `>` matches ONE OR MORE tokens (must be at end)");
    println!("• Patterns are case-sensitive");
    println!("• Empty tokens not allowed (e.g., orders..new)\n");

    println!("✓ Pattern matching example completed!");
    println!("Database persisted at: ./data/pattern_matching_example.db");

    std::process::exit(0);
}

