//! Publish/Subscribe example demonstrating topic-based messaging

use jrow_client::JrowClient;
use jrow_server::{from_typed_fn, JrowServer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};

#[derive(Deserialize, Serialize)]
struct EchoParams {
    message: String,
}

/// Start the pub/sub server
async fn start_server(server: Arc<JrowServer>) {
    // Add a simple echo handler
    let _echo_handler = from_typed_fn(|params: EchoParams| async move {
        println!("[SERVER] Echo: {}", params.message);
        Ok(serde_json::json!({"echoed": params.message}))
    });

    // Note: In a real application, you'd register handlers before building the server
    // This is just for demonstration

    println!("[SERVER] Starting pub/sub server on ws://127.0.0.1:9000");
    println!("[SERVER] Available topics for subscription:");
    println!("  - stock.prices    - Stock price updates");
    println!("  - weather.alerts  - Weather alerts");
    println!("  - chat.general    - General chat messages");
    println!();

    if let Err(e) = server.run().await {
        eprintln!("[SERVER] Error: {}", e);
    }
}

/// Publisher task - publishes periodic updates to topics
async fn publisher_task(server: Arc<JrowServer>) {
    sleep(Duration::from_millis(500)).await; // Wait for clients to connect

    let mut ticker = interval(Duration::from_secs(2));
    let mut counter = 0;

    loop {
        ticker.tick().await;
        counter += 1;

        // Publish stock price
        let stock_data = serde_json::json!({
            "symbol": "AAPL",
            "price": 150.0 + (counter as f64 * 0.5),
            "timestamp": counter
        });

        match server.publish("stock.prices", stock_data).await {
            Ok(count) => println!("[PUBLISHER] Published stock price to {} subscribers", count),
            Err(e) => eprintln!("[PUBLISHER] Error publishing: {}", e),
        }

        // Every 4 seconds, publish weather alert
        if counter % 2 == 0 {
            let weather_data = serde_json::json!({
                "alert": "Sunny day ahead",
                "temperature": 72 + counter,
                "timestamp": counter
            });

            match server.publish("weather.alerts", weather_data).await {
                Ok(count) => println!(
                    "[PUBLISHER] Published weather alert to {} subscribers",
                    count
                ),
                Err(e) => eprintln!("[PUBLISHER] Error publishing: {}", e),
            }
        }

        // Every 3 seconds, publish chat message
        if counter % 3 == 0 {
            let chat_data = serde_json::json!({
                "user": "System",
                "message": format!("Broadcast message #{}", counter / 3),
                "timestamp": counter
            });

            match server.publish("chat.general", chat_data).await {
                Ok(count) => println!(
                    "[PUBLISHER] Published chat message to {} subscribers",
                    count
                ),
                Err(e) => eprintln!("[PUBLISHER] Error publishing: {}", e),
            }
        }

        if counter >= 10 {
            println!("[PUBLISHER] Finished publishing");
            break;
        }
    }
}

/// Run a client that subscribes to topics
async fn run_client(client_id: u32, topics: Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("[CLIENT-{}] Connecting...", client_id);

    // Give the server a moment to start
    sleep(Duration::from_millis(100)).await;

    let client = JrowClient::connect("ws://127.0.0.1:9000").await?;
    println!("[CLIENT-{}] Connected!", client_id);

    // Subscribe to topics
    for topic in &topics {
        let topic_name = topic.to_string();
        let client_id_copy = client_id;

        client
            .subscribe(*topic, move |data| {
                let topic = topic_name.clone();
                async move {
                    println!(
                        "[CLIENT-{}] Received on '{}': {}",
                        client_id_copy, topic, data
                    );
                }
            })
            .await?;
        
        println!("[CLIENT-{}] Subscribed to '{}'", client_id, topic);
    }
    
    // Wait to receive messages
    sleep(Duration::from_secs(6)).await;
    
    // Unsubscribe from first topic (if any)
    if let Some(topic) = topics.first() {
        println!("[CLIENT-{}] Unsubscribing from '{}'", client_id, topic);
        client.unsubscribe(*topic).await?;
        println!("[CLIENT-{}] Unsubscribed from '{}'", client_id, topic);
    }

    // Wait a bit more
    sleep(Duration::from_secs(6)).await;

    // Show remaining subscriptions
    let subs = client.subscriptions().await;
    println!("[CLIENT-{}] Still subscribed to: {:?}", client_id, subs);

    println!("[CLIENT-{}] Disconnecting", client_id);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== JSON-RPC Pub/Sub Example ===\n");

    // Build the server
    let server = Arc::new(
        JrowServer::builder()
            .bind_str("127.0.0.1:9000")?
            .build()
            .await?,
    );

    // Start server in background
    let server_handle = {
        let server = Arc::clone(&server);
        tokio::spawn(async move {
            start_server(server).await;
        })
    };

    // Wait for server to start
    sleep(Duration::from_millis(100)).await;

    // Start publisher task
    let publisher_handle = {
        let server = Arc::clone(&server);
        tokio::spawn(async move {
            publisher_task(server).await;
        })
    };

    // Start multiple clients with different subscriptions
    let client1_handle = tokio::spawn(async move {
        if let Err(e) = run_client(1, vec!["stock.prices", "weather.alerts"]).await {
            eprintln!("[CLIENT-1] Error: {}", e);
        }
    });

    let client2_handle = tokio::spawn(async move {
        if let Err(e) = run_client(2, vec!["stock.prices", "chat.general"]).await {
            eprintln!("[CLIENT-2] Error: {}", e);
        }
    });

    let client3_handle = tokio::spawn(async move {
        if let Err(e) = run_client(3, vec!["weather.alerts", "chat.general"]).await {
            eprintln!("[CLIENT-3] Error: {}", e);
        }
    });

    // Wait for publisher to finish
    let _ = publisher_handle.await;

    // Wait for clients to finish
    let _ = tokio::join!(client1_handle, client2_handle, client3_handle);

    println!("\n[MAIN] All clients disconnected");
    println!("[MAIN] Shutting down server...");

    // Abort server
    server_handle.abort();

    println!("[MAIN] Done!");
    Ok(())
}
