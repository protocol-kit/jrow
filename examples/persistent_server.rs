//! Persistent pub/sub server example
//! 
//! This example starts a server that publishes messages to the "events" topic
//! every 10 seconds. Run this along with `persistent_client.rs` to see
//! persistent subscriptions in action.

use jrow_server::{JrowServer, RetentionPolicy};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Starting persistent pub/sub server...\n");

    // Start server with persistent storage
    let server = JrowServer::builder()
        .bind_str("127.0.0.1:9004")
        .unwrap()
        .with_persistent_storage("./data/persistent_events.db")
        .register_topic(
            "events",
            RetentionPolicy {
                max_age: Some(Duration::from_secs(3600)), // 1 hour
                max_count: Some(1000),
                max_bytes: Some(10 * 1024 * 1024), // 10MB
            },
        )
        .subscription_timeout(Duration::from_secs(300)) // 5 minutes
        .retention_interval(Duration::from_secs(60))
        .build()
        .await?;

    println!("✓ Server listening on ws://127.0.0.1:9004");
    println!("✓ Persistent storage: ./data/persistent_events.db");
    println!("✓ Publishing to 'events' topic every 10 seconds\n");

    // Spawn server in background
    let server = Arc::new(server);
    let server_clone = Arc::clone(&server);
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server_clone.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Shared event_id for graceful tracking
    let event_id = Arc::new(Mutex::new(1u64));
    let event_id_clone = Arc::clone(&event_id);

    // Spawn publishing task
    let server_clone = Arc::clone(&server);
    let publish_handle = tokio::spawn(async move {
        loop {
            let mut current_id = event_id_clone.lock().await;
            let id = *current_id;
            *current_id += 1;
            drop(current_id);

            let timestamp = chrono::Utc::now();
            let event_data = serde_json::json!({
                "event_id": id,
                "type": "system.event",
                "timestamp": timestamp.to_rfc3339(),
                "data": {
                    "message": format!("Event #{}", id),
                    "severity": if id % 3 == 0 { "high" } else { "normal" }
                }
            });

            match server_clone.publish_persistent("events", event_data).await {
                Ok(seq) => {
                    println!(
                        "[{}] Published event #{} (sequence: {})",
                        timestamp.format("%H:%M:%S"),
                        id,
                        seq
                    );
                }
                Err(e) => {
                    eprintln!("Failed to publish event: {}", e);
                    break;
                }
            }

            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });

    // Wait for Ctrl+C
    println!("Press Ctrl+C to stop\n");
    println!("═══════════════════════════════════════════════════════\n");

    tokio::signal::ctrl_c().await?;
    
    println!("\n\n🛑 Shutting down gracefully...");
    
    // Cancel publishing task
    publish_handle.abort();
    
    // Cancel server task
    server_handle.abort();
    
    // Give time for cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let final_event_id = *event_id.lock().await;
    println!("✓ Published {} events total", final_event_id - 1);
    println!("✓ Persistent storage saved at: ./data/persistent_events.db");
    println!("✓ Server stopped");
    
    Ok(())
}

