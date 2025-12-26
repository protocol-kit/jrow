//! Server for the reconnection example
//!
//! This server demonstrates automatic reconnection by accepting connections,
//! handling requests, and publishing periodic status updates.
//!
//! Run this server first:
//! ```bash
//! cargo run --example reconnection_server
//! ```
//!
//! Then run the client:
//! ```bash
//! cargo run --example reconnection
//! ```
//!
//! Try stopping and restarting this server to see the client reconnect.

use jrow_core::Result;
use jrow_server::{from_typed_fn, JrowServer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{interval, Duration};

#[derive(Deserialize)]
struct PingParams {
    #[allow(dead_code)]
    message: String,
}

#[derive(Serialize)]
struct PingResult {
    response: String,
}

async fn ping_handler(params: PingParams) -> Result<PingResult> {
    println!("📨 Received ping: {}", params.message);
    Ok(PingResult {
        response: format!("Pong! Received: {}", params.message),
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Reconnection Server Example ===\n");

    let addr: std::net::SocketAddr = "127.0.0.1:9004"
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

    println!("✓ Server listening on ws://127.0.0.1:9004");
    println!("✓ Registered method: ping");
    println!("\nWaiting for connections...");
    println!("Try stopping and restarting this server to test reconnection.\n");

    // Spawn a task to publish periodic status updates
    let server_clone = server.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(10));
        let mut count = 1;
        loop {
            ticker.tick().await;
            let message = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "count": count,
                "message": "Server is alive"
            });

            match server_clone.publish("status", message).await {
                Ok(sent) => {
                    if sent > 0 {
                        println!("📢 Published status update to {} subscriber(s)", sent);
                    }
                }
                Err(e) => eprintln!("Error publishing: {}", e),
            }

            count += 1;
        }
    });

    // Run the server
    server.run().await
}

