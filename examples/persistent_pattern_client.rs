//! Client example for NATS-style pattern matching with persistent subscriptions
//!
//! This client subscribes to multiple patterns and processes messages.
//! It demonstrates automatic resume after disconnect.
//!
//! Usage:
//!   Terminal 1: cargo run --example persistent_pattern_server
//!   Terminal 2: cargo run --example persistent_pattern_client
//!
//! Try stopping (Ctrl+C) and restarting this client to see automatic resume!

use jrow_client::JrowClient;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== NATS Pattern Matching - Client ===\n");
    println!("Connecting to server at ws://127.0.0.1:9007...");

    // Connect to server
    let client = JrowClient::connect("ws://127.0.0.1:9007").await?;
    println!("✓ Connected\n");

    // Counters for received messages
    let orders_all_count = Arc::new(Mutex::new(0));
    let orders_deep_count = Arc::new(Mutex::new(0));
    let user_events_count = Arc::new(Mutex::new(0));
    let success_count = Arc::new(Mutex::new(0));

    println!("=== Subscription 1: All order events (orders.*) ===");
    let count_clone = Arc::clone(&orders_all_count);
    let client_clone = client.clone();
    let resumed_seq = client
        .subscribe_persistent("orders-all", "orders.*", move |msg| {
            let count = Arc::clone(&count_clone);
            let client = client_clone.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(topic), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("topic").and_then(|v| v.as_str()),
                        obj.get("data"),
                    ) {
                        *count.lock().await += 1;
                        println!(
                            "[orders.*] topic={} seq={} data={}",
                            topic,
                            seq_id,
                            serde_json::to_string(data).unwrap_or_default()
                        );
                        client.ack_persistent("orders-all", seq_id);
                    }
                }
            }
        })
        .await?;
    println!("✓ Subscribed to 'orders.*' (resumed from seq {})", resumed_seq);
    println!("  Matches: orders.new, orders.shipped, orders.cancelled");
    println!("  Does NOT match: orders, orders.new.express\n");

    println!("=== Subscription 2: All deep order events (orders.>) ===");
    let count_clone = Arc::clone(&orders_deep_count);
    let client_clone = client.clone();
    let resumed_seq = client
        .subscribe_persistent("orders-deep", "orders.>", move |msg| {
            let count = Arc::clone(&count_clone);
            let client = client_clone.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(topic), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("topic").and_then(|v| v.as_str()),
                        obj.get("data"),
                    ) {
                        *count.lock().await += 1;
                        println!(
                            "[orders.>] topic={} seq={} data={}",
                            topic,
                            seq_id,
                            serde_json::to_string(data).unwrap_or_default()
                        );
                        client.ack_persistent("orders-deep", seq_id);
                    }
                }
            }
        })
        .await?;
    println!("✓ Subscribed to 'orders.>' (resumed from seq {})", resumed_seq);
    println!("  Matches: orders.new, orders.shipped, orders.new.express, etc.");
    println!("  Does NOT match: orders (needs at least one token after)\n");

    println!("=== Subscription 3: All user events (events.user.>) ===");
    let count_clone = Arc::clone(&user_events_count);
    let client_clone = client.clone();
    let resumed_seq = client
        .subscribe_persistent("user-events", "events.user.>", move |msg| {
            let count = Arc::clone(&count_clone);
            let client = client_clone.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(topic), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("topic").and_then(|v| v.as_str()),
                        obj.get("data"),
                    ) {
                        *count.lock().await += 1;
                        println!(
                            "[events.user.>] topic={} seq={} data={}",
                            topic,
                            seq_id,
                            serde_json::to_string(data).unwrap_or_default()
                        );
                        client.ack_persistent("user-events", seq_id);
                    }
                }
            }
        })
        .await?;
    println!("✓ Subscribed to 'events.user.>' (resumed from seq {})", resumed_seq);
    println!("  Matches: events.user.login, events.user.login.success, etc.\n");

    println!("=== Subscription 4: All success events (*.*.success) ===");
    let count_clone = Arc::clone(&success_count);
    let client_clone = client.clone();
    let resumed_seq = client
        .subscribe_persistent("all-success", "*.*.success", move |msg| {
            let count = Arc::clone(&count_clone);
            let client = client_clone.clone();
            async move {
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(topic), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("topic").and_then(|v| v.as_str()),
                        obj.get("data"),
                    ) {
                        *count.lock().await += 1;
                        println!(
                            "[*.*.success] topic={} seq={} data={}",
                            topic,
                            seq_id,
                            serde_json::to_string(data).unwrap_or_default()
                        );
                        client.ack_persistent("all-success", seq_id);
                    }
                }
            }
        })
        .await?;
    println!("✓ Subscribed to '*.*.success' (resumed from seq {})", resumed_seq);
    println!("  Matches: events.user.success, payments.card.success, etc.");
    println!("  Requires exactly 3 tokens with 'success' at the end\n");

    println!("===================================");
    println!("Listening for messages...");
    println!("Press Ctrl+C to stop (state is persisted)");
    println!("Restart to see automatic resume!");
    println!("===================================\n");

    // Keep running and show statistics
    let mut last_total = 0;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        let orders_all = *orders_all_count.lock().await;
        let orders_deep = *orders_deep_count.lock().await;
        let user_events = *user_events_count.lock().await;
        let success = *success_count.lock().await;
        let total = orders_all + orders_deep + user_events + success;

        if total > last_total {
            println!("\n📊 Message Statistics:");
            println!("  orders.* → {} messages", orders_all);
            println!("  orders.> → {} messages", orders_deep);
            println!("  events.user.> → {} messages", user_events);
            println!("  *.*.success → {} messages", success);
            println!("  Total: {} messages\n", total);
            last_total = total;
        }
    }
}

