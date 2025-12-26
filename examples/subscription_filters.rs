//! Example demonstrating subscription filters with NATS-style patterns
//!
//! This example shows how to use NATS-style patterns to subscribe to multiple topics
//! with a single subscription. Clients can use wildcards (* and >) to match
//! multiple topics dynamically.
//!
//! Pattern syntax:
//! - `*` matches exactly one token (e.g., `events.*` matches `events.user` but not `events.user.login`)
//! - `>` matches one or more tokens (e.g., `events.>` matches `events.user` and `events.user.login`)
//!
//! Run this example:
//! ```bash
//! cargo run --example subscription_filters
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
    println!("=== Subscription Filters Example ===\n");

    // Start server
    let addr: std::net::SocketAddr = "127.0.0.1:9005"
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

    println!("✓ Server listening on ws://127.0.0.1:9005\n");

    // Spawn server task
    let server_clone = server.clone();
    tokio::spawn(async move {
        server_clone.run().await.ok();
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create clients with different subscription patterns
    println!("Creating clients with different subscription patterns...\n");

    // Client 1: Subscribe to all single-level events (events.*)
    let client1 = JrowClient::connect("ws://127.0.0.1:9005").await?;
    client1
        .subscribe("events.*", |msg| async move {
            if let Some(obj) = msg.as_object() {
                let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("unknown");
                let data = obj.get("data").unwrap_or(&msg);
                println!("[Client 1 - events.*] topic={}, data={}", topic, data);
            } else {
                println!("[Client 1 - events.*] Received (exact): {}", msg);
            }
        })
        .await?;
    println!("✓ Client 1 subscribed to 'events.*'");

    // Client 2: Subscribe to all multi-level events (events.>)
    let client2 = JrowClient::connect("ws://127.0.0.1:9005").await?;
    client2
        .subscribe("events.>", |msg| async move {
            if let Some(obj) = msg.as_object() {
                let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("unknown");
                let data = obj.get("data").unwrap_or(&msg);
                println!("[Client 2 - events.>] topic={}, data={}", topic, data);
            } else {
                println!("[Client 2 - events.>] Received (exact): {}", msg);
            }
        })
        .await?;
    println!("✓ Client 2 subscribed to 'events.>' (multi-wildcard)");

    // Client 3: Subscribe to exact topic
    let client3 = JrowClient::connect("ws://127.0.0.1:9005").await?;
    client3
        .subscribe("events.user.login", |msg| async move {
            println!("[Client 3 - exact] Received: {}", msg);
        })
        .await?;
    println!("✓ Client 3 subscribed to 'events.user.login' (exact)\n");

    // Client 4: Subscribe to logs.*.error (single-level wildcard in middle)
    let client4 = JrowClient::connect("ws://127.0.0.1:9005").await?;
    client4
        .subscribe("logs.*.error", |msg| async move {
            if let Some(obj) = msg.as_object() {
                let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("unknown");
                let data = obj.get("data").unwrap_or(&msg);
                println!("[Client 4 - logs.*.error] topic={}, data={}", topic, data);
            } else {
                println!("[Client 4 - logs.*.error] Received (exact): {}", msg);
            }
        })
        .await?;
    println!("✓ Client 4 subscribed to 'logs.*.error'\n");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish messages to various topics using batch publish
    println!("=== Publishing Messages (using batch) ===\n");

    let messages: Vec<(String, serde_json::Value)> = vec![
        ("events.user.login", "User Alice logged in"),
        ("events.user.logout", "User Alice logged out"),
        ("events.admin.login", "Admin Bob logged in"),
        ("logs.app.error", "Application error occurred"),
        ("logs.db.error", "Database connection failed"),
        ("logs.app.info", "Application started"),
        ("system.startup", "System initialized"),
    ]
    .into_iter()
    .map(|(topic, message)| {
        (
            topic.to_string(),
            serde_json::json!({
                "topic": topic,
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        )
    })
    .collect();

    println!("📢 Publishing {} messages in a single batch...\n", messages.len());
    let results = server.publish_batch(messages).await?;

    for (topic, count) in results {
        println!("  '{}' → {} subscriber(s)", topic, count);
    }
    println!();

    tokio::time::sleep(Duration::from_millis(500)).await;

    println!("=== Summary ===\n");
    println!("Pattern Matching Results:");
    println!("• Client 1 (events.*): Matches events.user, events.admin (single level only)");
    println!("• Client 2 (events.>): Matches events.user, events.user.login, events.admin.login (one or more levels)");
    println!("• Client 3 (exact): Received only events.user.login");
    println!("• Client 4 (logs.*.error): Matches logs.app.error, logs.db.error (single wildcard in middle)");
    println!("\nNATS Pattern Syntax:");
    println!("  * = matches exactly one token");
    println!("  > = matches one or more tokens (must be at the end)");
    println!("\nPress Ctrl+C to exit.");

    // Keep alive
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}

