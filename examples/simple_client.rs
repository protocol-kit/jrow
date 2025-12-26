//! Simple JSON-RPC client example

use jrow_client::JrowClient;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Deserialize)]
struct AddResult {
    sum: i32,
}

#[derive(Serialize)]
struct GreetParams {
    name: String,
}

#[derive(Deserialize)]
struct GreetResult {
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to JSON-RPC server at ws://127.0.0.1:8080");

    let client = JrowClient::connect("ws://127.0.0.1:8080").await?;

    println!("Connected! Sending requests...\n");

    // Call the add method
    let add_result: AddResult = client.request("add", AddParams { a: 5, b: 3 }).await?;
    println!("add(5, 3) = {}", add_result.sum);

    // Call the greet method
    let greet_result: GreetResult = client
        .request(
            "greet",
            GreetParams {
                name: "Alice".to_string(),
            },
        )
        .await?;
    println!("greet('Alice') = {}", greet_result.message);

    // Call the echo method
    let echo_result: serde_json::Value = client
        .request("echo", serde_json::json!({"test": "data", "number": 42}))
        .await?;
    println!("echo({{...}}) = {}", echo_result);

    // Send a notification (no response expected)
    println!("\nSending notification...");
    client
        .notify(
            "log",
            serde_json::json!({"level": "info", "message": "Test log"}),
        )
        .await?;

    println!("\nAll requests completed successfully!");

    // Keep the client alive for a moment
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}
