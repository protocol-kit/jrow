//! Example demonstrating automatic reconnection
//!
//! This example shows how to use the automatic reconnection feature.
//! It creates a client with reconnection enabled, connects to a server,
//! and demonstrates what happens when the connection is lost.
//!
//! Run the server first:
//! ```bash
//! cargo run --example reconnection_server
//! ```
//!
//! Then run this client:
//! ```bash
//! cargo run --example reconnection_client
//! ```
//!
//! Try stopping and restarting the server to see reconnection in action.

use jrow_client::{ClientBuilder, ExponentialBackoff};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Automatic Reconnection Example ===\n");

    // Create a client with automatic reconnection
    let strategy = ExponentialBackoff::new(Duration::from_secs(1), Duration::from_secs(10))
        .with_max_attempts(5)
        .with_jitter();

    println!("Connecting to server with automatic reconnection...");
    let client = ClientBuilder::new("ws://127.0.0.1:9004")
        .with_reconnect(Box::new(strategy))
        .connect()
        .await?;

    println!("✓ Connected successfully!\n");

    // Subscribe to a topic
    println!("Subscribing to 'status' topic...");
    client
        .subscribe("status", |msg| async move {
            println!("📩 Received status update: {}", msg);
        })
        .await?;
    println!("✓ Subscribed\n");

    // Send some requests
    println!("Sending requests...");
    for i in 1..=3 {
        match client.request::<_, String>("ping", format!("Message {}", i)).await {
            Ok(response) => println!("✓ Response {}: {}", i, response),
            Err(e) => println!("✗ Error: {}", e),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    println!("\n=== Connection Status ===");
    if let Some(state) = client.connection_state().await {
        println!("Current state: {:?}", state);
    }

    // Keep the client alive to demonstrate reconnection
    println!("\n=== Monitoring Connection ===");
    println!("Try stopping and restarting the server to see reconnection in action.");
    println!("The client will automatically reconnect and resubscribe to topics.");
    println!("Press Ctrl+C to exit.\n");

    // Periodically send ping requests
    let mut counter = 4;
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        if let Some(state) = client.connection_state().await {
            println!("[{}] Connection state: {:?}", counter, state);
        }

        match client.request::<_, String>("ping", format!("Message {}", counter)).await {
            Ok(response) => println!("[{}] ✓ {}", counter, response),
            Err(e) => println!("[{}] ✗ Error: {}", counter, e),
        }

        counter += 1;
    }
}

