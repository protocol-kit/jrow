//! Simple JSON-RPC server example

use jrow_server::{from_typed_fn, JrowServer};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Serialize)]
struct AddResult {
    sum: i32,
}

#[derive(Deserialize)]
struct GreetParams {
    name: String,
}

#[derive(Serialize)]
struct GreetResult {
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting JSON-RPC server on ws://127.0.0.1:8080");

    // Create handlers
    let add_handler = from_typed_fn(|params: AddParams| async move {
        Ok(AddResult {
            sum: params.a + params.b,
        })
    });

    let greet_handler = from_typed_fn(|params: GreetParams| async move {
        Ok(GreetResult {
            message: format!("Hello, {}!", params.name),
        })
    });

    let echo_handler = from_typed_fn(|params: serde_json::Value| async move { Ok(params) });

    // Build and run the server
    let server = JrowServer::builder()
        .bind_str("127.0.0.1:8080")?
        .handler("add", add_handler)
        .handler("greet", greet_handler)
        .handler("echo", echo_handler)
        .build()
        .await?;

    println!("Server is running. Available methods:");
    println!("  - add(a, b): Add two numbers");
    println!("  - greet(name): Greet someone");
    println!("  - echo(value): Echo back any value");
    println!("\nPress Ctrl+C to stop");

    server.run().await?;

    Ok(())
}
