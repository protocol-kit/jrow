//! Persistent pub/sub client example
//! 
//! This example connects to the persistent_server and subscribes to events.
//! It demonstrates:
//! - Persistent subscription with exactly-once delivery
//! - Automatic resume from last acknowledged position
//! - Graceful reconnection handling
//!
//! Usage:
//! 1. Start the server: cargo run --example persistent_server
//! 2. Start this client: cargo run --example persistent_client
//! 3. Stop and restart the client to see it resume from where it left off

use jrow_client::JrowClient;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🔌 Connecting to persistent pub/sub server...\n");

    // Connect to server
    let client = JrowClient::connect("ws://127.0.0.1:9004").await?;
    println!("✓ Connected to ws://127.0.0.1:9004\n");

    // Track processed events
    let processed_count = Arc::new(Mutex::new(0));
    let processed_clone = Arc::clone(&processed_count);

    // Subscribe with a unique subscription ID
    // This allows us to resume from the same position across restarts
    let subscription_id = "event-processor-1";
    let client_clone = client.clone();

    println!("📡 Subscribing to 'events' topic with subscription ID: {}\n", subscription_id);

    let resumed_seq = client
        .subscribe_persistent(subscription_id, "events", move |msg| {
            let client = client_clone.clone();
            let processed = Arc::clone(&processed_clone);
            async move {
                // Extract sequence_id and data from the persistent message format
                if let Some(obj) = msg.as_object() {
                    if let (Some(seq_id), Some(data)) = (
                        obj.get("sequence_id").and_then(|v| v.as_u64()),
                        obj.get("data"),
                    ) {
                        // Extract event details
                        let event_id = data
                            .get("event_id")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let event_type = data
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let timestamp = data
                            .get("timestamp")
                            .and_then(|v| v.as_str())
                            .unwrap_or("N/A");
                        let message = data
                            .get("data")
                            .and_then(|d| d.get("message"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("N/A");
                        let severity = data
                            .get("data")
                            .and_then(|d| d.get("severity"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("normal");

                        // Display event with emoji based on severity
                        let emoji = if severity == "high" { "🔴" } else { "🟢" };
                        println!(
                            "{} [seq:{}] Event #{}: {} | {} | {}",
                            emoji, seq_id, event_id, event_type, message, timestamp
                        );

                        // Simulate processing time
                        tokio::time::sleep(Duration::from_millis(100)).await;

                        // Acknowledge the message
                        // This is fire-and-forget, spawned internally to prevent blocking
                        client.ack_persistent(subscription_id, seq_id);

                        let mut count = processed.lock().await;
                        *count += 1;
                    }
                }
            }
        })
        .await?;

    println!(
        "✓ Subscribed! Resumed from sequence: {}\n",
        resumed_seq
    );

    if resumed_seq > 0 {
        println!("ℹ️  Catching up on unprocessed events...\n");
    } else {
        println!("ℹ️  Waiting for new events...\n");
    }

    // Keep the client running
    println!("Press Ctrl+C to stop\n");
    println!("═══════════════════════════════════════════════════════\n");

    // Periodically show stats
    let stats_task = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let count = processed_count.lock().await;
            if *count > 0 {
                println!("\n📊 Stats: {} events processed", *count);
                println!("═══════════════════════════════════════════════════════\n");
            }
        }
    });

    // Wait for interrupt signal
    tokio::signal::ctrl_c().await?;
    stats_task.abort();

    println!("\n\n👋 Shutting down...");
    println!("ℹ️  Your progress has been saved. Restart to resume from where you left off.");

    Ok(())
}



