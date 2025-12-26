use jrow_client::JrowClient;
use jrow_server::{from_typed_fn, JrowServer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Event {
    timestamp: u64,
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Server Batch Publish Example ===\n");

    // Build server
    let ping_handler = from_typed_fn(|_: ()| async { Ok("pong") });

    let server = Arc::new(
        JrowServer::builder()
            .bind("127.0.0.1:9004".parse::<std::net::SocketAddr>()?)
            .handler("ping", ping_handler)
            .build()
            .await?,
    );

    // Start server in background
    let server_clone = Arc::clone(&server);
    let _server_handle = tokio::spawn(async move {
        if let Err(e) = server_clone.run().await {
            eprintln!("[SERVER] Error: {}", e);
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Connect clients
    println!("Connecting 3 clients...");
    let client1 = JrowClient::connect("ws://127.0.0.1:9004").await?;
    let client2 = JrowClient::connect("ws://127.0.0.1:9004").await?;
    let client3 = JrowClient::connect("ws://127.0.0.1:9004").await?;
    println!("✓ 3 clients connected\n");

    // Subscribe clients to different topics
    println!("Setting up subscriptions...");

    // Client 1: Subscribe to news and alerts
    client1
        .subscribe("news", |data: serde_json::Value| async move {
            if let Ok(event) = serde_json::from_value::<Event>(data) {
                println!("[CLIENT-1] 📰 NEWS: {}", event.message);
            }
        })
        .await?;
    client1
        .subscribe("alerts", |data: serde_json::Value| async move {
            if let Ok(event) = serde_json::from_value::<Event>(data) {
                println!("[CLIENT-1] 🚨 ALERT: {}", event.message);
            }
        })
        .await?;

    // Client 2: Subscribe to news and updates
    client2
        .subscribe("news", |data: serde_json::Value| async move {
            if let Ok(event) = serde_json::from_value::<Event>(data) {
                println!("[CLIENT-2] 📰 NEWS: {}", event.message);
            }
        })
        .await?;
    client2
        .subscribe("updates", |data: serde_json::Value| async move {
            if let Ok(event) = serde_json::from_value::<Event>(data) {
                println!("[CLIENT-2] 🔔 UPDATE: {}", event.message);
            }
        })
        .await?;

    // Client 3: Subscribe to all topics
    client3
        .subscribe("news", |data: serde_json::Value| async move {
            if let Ok(event) = serde_json::from_value::<Event>(data) {
                println!("[CLIENT-3] 📰 NEWS: {}", event.message);
            }
        })
        .await?;
    client3
        .subscribe("alerts", |data: serde_json::Value| async move {
            if let Ok(event) = serde_json::from_value::<Event>(data) {
                println!("[CLIENT-3] 🚨 ALERT: {}", event.message);
            }
        })
        .await?;
    client3
        .subscribe("updates", |data: serde_json::Value| async move {
            if let Ok(event) = serde_json::from_value::<Event>(data) {
                println!("[CLIENT-3] 🔔 UPDATE: {}", event.message);
            }
        })
        .await?;

    println!("✓ All clients subscribed\n");
    sleep(Duration::from_millis(200)).await;

    // Publish to multiple topics using batch
    println!("--- Batch Publish ---");
    let messages = vec![
        (
            "news".to_string(),
            serde_json::to_value(Event {
                timestamp: 1000,
                message: "New feature released!".to_string(),
            })?,
        ),
        (
            "alerts".to_string(),
            serde_json::to_value(Event {
                timestamp: 1001,
                message: "System maintenance scheduled".to_string(),
            })?,
        ),
        (
            "updates".to_string(),
            serde_json::to_value(Event {
                timestamp: 1002,
                message: "Version 2.0 is available".to_string(),
            })?,
        ),
    ];

    let results = server.publish_batch(messages).await?;

    println!("\nPublish results:");
    for (topic, count) in &results {
        println!("  '{}': {} subscribers notified", topic, count);
    }

    sleep(Duration::from_millis(500)).await;

    // Compare performance: individual vs batch
    println!("\n--- Performance Comparison ---\n");

    // Individual publishes
    println!("Publishing to 10 topics individually...");
    let start = std::time::Instant::now();
    for i in 0..10 {
        let topic = format!("test-{}", i);
        server.publish(&topic, serde_json::json!({"id": i})).await?;
    }
    let individual_time = start.elapsed();
    println!("  Individual publish (10 topics): {:?}", individual_time);

    // Batch publish
    println!("Publishing to 10 topics in a single batch...");
    let batch_messages: Vec<(String, serde_json::Value)> = (0..10)
        .map(|i| (format!("test-{}", i), serde_json::json!({"id": i})))
        .collect();

    let start = std::time::Instant::now();
    server.publish_batch(batch_messages).await?;
    let batch_time = start.elapsed();
    println!("  Batch publish (10 topics):      {:?}", batch_time);

    if batch_time.as_micros() > 0 {
        println!(
            "\n  ⚡ Speed improvement: {:.2}x faster",
            individual_time.as_secs_f64() / batch_time.as_secs_f64()
        );
    }

    println!("\n✓ Example completed successfully");
    Ok(())
}
