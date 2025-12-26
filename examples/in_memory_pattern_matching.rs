//! Example demonstrating NATS-style pattern matching for in-memory subscriptions
//!
//! This example shows how to use NATS-style patterns for in-memory pub/sub.
//! It demonstrates all three subscription types: exact, single wildcard (*), and multi wildcard (>).
//!
//! Run this example:
//! ```bash
//! cargo run --example in_memory_pattern_matching
//! ```

use jrow_client::JrowClient;
use jrow_core::Result;
use jrow_server::{from_typed_fn, JrowServer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::Duration;

#[derive(Deserialize)]
struct PingParams {}

#[derive(Serialize)]
struct PingResult {
    message: String,
}

async fn ping_handler(_params: PingParams) -> Result<PingResult> {
    Ok(PingResult {
        message: "pong".to_string(),
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== In-Memory NATS Pattern Matching Example ===\n");

    // Start server
    let addr: std::net::SocketAddr = "127.0.0.1:9008"
        .parse()
        .map_err(|e: std::net::AddrParseError| {
            jrow_core::Error::Internal(format!("Failed to parse address: {}", e))
        })?;

    let server = Arc::new(
        JrowServer::builder()
            .bind(addr)
            .handler("ping", from_typed_fn(ping_handler))
            .build()
            .await?,
    );

    println!("✓ Server listening on ws://127.0.0.1:9008\n");

    // Spawn server task
    let server_clone = server.clone();
    tokio::spawn(async move {
        server_clone.run().await.ok();
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("=== Setting up subscriptions ===\n");

    // Subscription 1: Exact match - orders.new
    let client1 = JrowClient::connect("ws://127.0.0.1:9008").await?;
    client1
        .subscribe("orders.new", |msg| async move {
            println!("  [EXACT: orders.new] {}", msg);
        })
        .await?;
    println!("✓ Client 1: Exact subscription to 'orders.new'");

    // Subscription 2: Single wildcard - orders.*
    let client2 = JrowClient::connect("ws://127.0.0.1:9008").await?;
    client2
        .subscribe("orders.*", |msg| async move {
            if let Some(obj) = msg.as_object() {
                let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("?");
                let data = obj.get("data").unwrap_or(&msg);
                println!("  [PATTERN: orders.*] topic={} data={}", topic, data);
            }
        })
        .await?;
    println!("✓ Client 2: Pattern 'orders.*' (matches one level)");

    // Subscription 3: Multi wildcard - orders.>
    let client3 = JrowClient::connect("ws://127.0.0.1:9008").await?;
    client3
        .subscribe("orders.>", |msg| async move {
            if let Some(obj) = msg.as_object() {
                let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("?");
                let data = obj.get("data").unwrap_or(&msg);
                println!("  [PATTERN: orders.>] topic={} data={}", topic, data);
            }
        })
        .await?;
    println!("✓ Client 3: Pattern 'orders.>' (matches one or more levels)");

    // Subscription 4: events.user.>
    let client4 = JrowClient::connect("ws://127.0.0.1:9008").await?;
    client4
        .subscribe("events.user.>", |msg| async move {
            if let Some(obj) = msg.as_object() {
                let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("?");
                let data = obj.get("data").unwrap_or(&msg);
                println!("  [PATTERN: events.user.>] topic={} data={}", topic, data);
            }
        })
        .await?;
    println!("✓ Client 4: Pattern 'events.user.>' (deep matching)");

    // Subscription 5: *.*.success
    let client5 = JrowClient::connect("ws://127.0.0.1:9008").await?;
    client5
        .subscribe("*.*.success", |msg| async move {
            if let Some(obj) = msg.as_object() {
                let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("?");
                let data = obj.get("data").unwrap_or(&msg);
                println!("  [PATTERN: *.*.success] topic={} data={}", topic, data);
            }
        })
        .await?;
    println!("✓ Client 5: Pattern '*.*.success' (multiple single wildcards)\n");

    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("=== Publishing Messages ===\n");

    // Test Case 1: Exact match
    println!("1. Publishing to 'orders.new':");
    let count = server
        .publish(
            "orders.new",
            serde_json::json!({"order_id": 123, "item": "Widget"}),
        )
        .await?;
    println!("  Delivered to {} subscriber(s)", count);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test Case 2: Single level match
    println!("\n2. Publishing to 'orders.shipped':");
    let count = server
        .publish(
            "orders.shipped",
            serde_json::json!({"order_id": 124, "tracking": "ABC123"}),
        )
        .await?;
    println!("  Delivered to {} subscriber(s)", count);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test Case 3: Multi-level match
    println!("\n3. Publishing to 'orders.new.express':");
    let count = server
        .publish(
            "orders.new.express",
            serde_json::json!({"order_id": 125, "priority": "high"}),
        )
        .await?;
    println!("  Delivered to {} subscriber(s)", count);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test Case 4: Deep nested topic
    println!("\n4. Publishing to 'events.user.login':");
    let count = server
        .publish(
            "events.user.login",
            serde_json::json!({"user_id": "alice", "timestamp": "2024-01-01"}),
        )
        .await?;
    println!("  Delivered to {} subscriber(s)", count);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test Case 5: Deeper nested topic
    println!("\n5. Publishing to 'events.user.login.success':");
    let count = server
        .publish(
            "events.user.login.success",
            serde_json::json!({"user_id": "bob", "method": "oauth"}),
        )
        .await?;
    println!("  Delivered to {} subscriber(s)", count);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test Case 6: Multiple wildcards match
    println!("\n6. Publishing to 'payments.card.success':");
    server
        .publish(
            "payments.card.success",
            serde_json::json!({"amount": 99.99, "currency": "USD"}),
        )
        .await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test Case 7: No match
    println!("\n7. Publishing to 'other.topic' (should have no subscribers):");
    let count = server
        .publish("other.topic", serde_json::json!({"data": "test"}))
        .await?;
    println!("  Delivered to {} subscriber(s)", count);
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("\n=== Pattern Matching Summary ===\n");
    println!("Subscriptions:");
    println!("  1. 'orders.new' (exact)      → Matches only orders.new");
    println!("  2. 'orders.*'                → Matches orders.new, orders.shipped");
    println!("  3. 'orders.>'                → Matches orders.new, orders.shipped, orders.new.express");
    println!("  4. 'events.user.>'           → Matches events.user.login, events.user.login.success");
    println!("  5. '*.*.success'             → Matches events.user.success, payments.card.success");
    println!();
    println!("Pattern Rules:");
    println!("  • '*' matches exactly ONE token");
    println!("  • '>' matches ONE OR MORE tokens (must be at the end)");
    println!("  • Tokens are separated by '.'");
    println!("  • Multiple '*' wildcards are allowed in a pattern");
    println!("  • Only one '>' wildcard is allowed, and it must be the last token");

    // Keep alive briefly
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(())
}

