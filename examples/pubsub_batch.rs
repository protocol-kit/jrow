use jrow_client::JrowClient;
use jrow_server::{from_typed_fn, JrowServer};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Serialize, Deserialize)]
struct Event {
    timestamp: u64,
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Batch Subscribe/Unsubscribe Example ===\n");

    // Build and start server
    let ping_handler = from_typed_fn(|_: ()| async { Ok("pong") });

    let server = JrowServer::builder()
        .bind("127.0.0.1:9003".parse::<std::net::SocketAddr>()?)
        .handler("ping", ping_handler)
        .build()
        .await?;

    // Start server in background (consumes server)
    let _server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("[SERVER] Error: {}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = JrowClient::connect("ws://127.0.0.1:9003").await?;
    println!("✓ Client connected\n");

    // Subscribe to multiple topics at once using batch
    println!("--- Batch Subscribe ---");

    // Helper function to create handlers
    fn make_handler(
        prefix: &'static str,
        emoji: &'static str,
    ) -> impl Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
           + Send
           + Sync
           + 'static
           + Clone {
        move |value: serde_json::Value| {
            Box::pin(async move {
                if let Ok(event) = serde_json::from_value::<Event>(value) {
                    println!(
                        "{} {}: {} (ts: {})",
                        emoji, prefix, event.message, event.timestamp
                    );
                }
            })
        }
    }

    let topics = vec![
        ("news".to_string(), make_handler("NEWS", "📰")),
        ("alerts".to_string(), make_handler("ALERT", "🚨")),
        ("updates".to_string(), make_handler("UPDATE", "🔔")),
    ];

    client.subscribe_batch(topics).await?;
    println!("✓ Subscribed to 3 topics in a single batch request\n");

    sleep(Duration::from_millis(200)).await;

    // Unsubscribe from multiple topics at once using batch
    println!("--- Batch Unsubscribe ---");
    client
        .unsubscribe_batch(vec![
            "news".to_string(),
            "alerts".to_string(),
            "updates".to_string(),
        ])
        .await?;
    println!("✓ Unsubscribed from 3 topics in a single batch request\n");

    sleep(Duration::from_millis(100)).await;

    // Performance comparison: individual vs batch
    println!("--- Performance Comparison ---\n");

    // Subscribe individually (10 topics)
    println!("Subscribing to 10 topics individually...");
    let start = std::time::Instant::now();
    for i in 0..10 {
        let topic = format!("topic-{}", i);
        client
            .subscribe(&topic, |_: serde_json::Value| async move {})
            .await?;
    }
    let individual_time = start.elapsed();
    println!("  Individual subscribe (10 topics): {:?}", individual_time);

    // Unsubscribe individually
    for i in 0..10 {
        client.unsubscribe(format!("topic-{}", i)).await?;
    }

    sleep(Duration::from_millis(100)).await;

    // Subscribe using batch (10 topics)
    println!("Subscribing to 10 topics in a single batch...");
    let start = std::time::Instant::now();
    let batch_topics: Vec<(String, _)> = (0..10)
        .map(|i| {
            (format!("topic-{}", i), |_: serde_json::Value| {
                Box::pin(async move {})
                    as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            })
        })
        .collect();
    client.subscribe_batch(batch_topics).await?;
    let batch_time = start.elapsed();
    println!("  Batch subscribe (10 topics):      {:?}", batch_time);

    if batch_time.as_micros() > 0 {
        println!(
            "\n  ⚡ Speed improvement: {:.2}x faster",
            individual_time.as_secs_f64() / batch_time.as_secs_f64()
        );
    }

    println!("\n✓ Example completed successfully");
    Ok(())
}
