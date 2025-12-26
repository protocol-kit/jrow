//! JROW Client
//!
//! This is an example client that connects to the server and demonstrates
//! making various types of JSON-RPC requests.
//!
//! # What This Demonstrates
//!
//! - **Connection**: Establishing WebSocket connection to server
//! - **Requests**: Making RPC calls with typed parameters
//! - **Type Safety**: Using typed request/response structures
//! - **Notifications**: Sending fire-and-forget messages
//!
//! # Configuration
//!
//! - Server URL: Set via `SERVER_URL` environment variable
//! - Default: `ws://127.0.0.1:8080`
//!
//! # Running
//!
//! ```bash
//! # Start the server first
//! cargo run --bin server
//!
//! # In another terminal, run the client
//! cargo run --bin client
//!
//! # Or with custom server URL
//! SERVER_URL=ws://example.com:8080 cargo run --bin client
//! ```
//!
//! # Extending
//!
//! To add more examples:
//! 1. Define types in `src/types.rs`
//! 2. Add handler in `src/handlers.rs`
//! 3. Register in `bin/server.rs`
//! 4. Call from this client

use jrow_client::ClientBuilder;
use my_jrow_app::{AddParams, AddResult, EchoParams, EchoResult, StatusParams, StatusResult};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse server URL from environment or use default
    let server_url = std::env::var("SERVER_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8080".to_string());

    println!("Connecting to {}", server_url);

    // Connect to the server
    let client = ClientBuilder::new(&server_url)
        // Optional: Enable automatic reconnection
        // .with_default_reconnect()
        // Optional: Enable observability
        // .with_default_observability()
        .connect()
        .await?;

    println!("Connected! Running examples...\n");

    // Example 1: Add two numbers
    println!("1. Adding numbers:");
    let add_result: AddResult = client
        .request("add", AddParams { a: 5, b: 3 })
        .await?;
    println!("   5 + 3 = {}\n", add_result.sum);

    // Example 2: Echo a message
    println!("2. Echo message:");
    let echo_result: EchoResult = client
        .request("echo", EchoParams {
            message: "Hello, JROW!".to_string(),
        })
        .await?;
    println!("   Echo: {}\n", echo_result.message);

    // Example 3: Get server status
    println!("3. Server status:");
    let status: StatusResult = client
        .request("status", StatusParams {})
        .await?;
    println!("   Status: {}", status.status);
    println!("   Uptime: {} seconds\n", status.uptime_seconds);

    // Example 4: Notification (no response expected)
    println!("4. Sending notification:");
    client.notify("log", serde_json::json!({
        "level": "info",
        "message": "Client example completed"
    })).await?;
    println!("   Notification sent\n");

    println!("All examples completed successfully!");

    Ok(())
}



