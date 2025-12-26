//! Bidirectional communication example with notifications

use jrow_client::JrowClient;
use jrow_server::{from_typed_fn, JrowServer};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Deserialize, Serialize)]
struct LogParams {
    level: String,
    message: String,
}

#[derive(Deserialize, Serialize)]
struct StatusUpdate {
    status: String,
    timestamp: u64,
}

/// Start the server in a background task
async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    let log_handler = from_typed_fn(|params: LogParams| async move {
        println!(
            "[SERVER] Received log: [{}] {}",
            params.level, params.message
        );
        Ok(serde_json::json!({"received": true}))
    });

    let server = JrowServer::builder()
        .bind_str("127.0.0.1:8081")?
        .handler("log", log_handler)
        .build()
        .await?;

    println!("[SERVER] Started on ws://127.0.0.1:8081");

    server.run().await?;
    Ok(())
}

/// Run the client
async fn run_client() -> Result<(), Box<dyn std::error::Error>> {
    println!("[CLIENT] Connecting to server...");

    // Give the server a moment to start
    sleep(Duration::from_millis(100)).await;

    let client = JrowClient::connect("ws://127.0.0.1:8081").await?;
    println!("[CLIENT] Connected!");

    // Register a handler for incoming notifications
    client
        .on_notification("status_update", |notif| async move {
            if let Some(params) = notif.params {
                if let Ok(update) = serde_json::from_value::<StatusUpdate>(params) {
                    println!(
                        "[CLIENT] Received status update: {} at {}",
                        update.status, update.timestamp
                    );
                }
            }
        })
        .await;

    println!("[CLIENT] Registered notification handler");

    // Send some log requests
    for i in 1..=3 {
        let result: serde_json::Value = client
            .request(
                "log",
                LogParams {
                    level: "info".to_string(),
                    message: format!("Test message {}", i),
                },
            )
            .await?;
        println!("[CLIENT] Log response: {}", result);
        sleep(Duration::from_millis(500)).await;
    }

    // Send a notification
    println!("[CLIENT] Sending notification to server...");
    client
        .notify(
            "client_event",
            serde_json::json!({"event": "test", "data": "hello"}),
        )
        .await?;

    println!("[CLIENT] Keeping connection alive for 2 seconds...");
    sleep(Duration::from_secs(2)).await;

    println!("[CLIENT] Done!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Bidirectional JSON-RPC Example ===\n");

    // Start server in background
    tokio::spawn(async {
        if let Err(e) = start_server().await {
            eprintln!("[SERVER] Error: {}", e);
        }
    });

    // Run client
    run_client().await?;

    Ok(())
}
