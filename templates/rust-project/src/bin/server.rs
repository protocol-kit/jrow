//! JROW Server
//!
//! This is the main server binary. It sets up the JSON-RPC server,
//! registers method handlers, and listens for incoming WebSocket connections.
//!
//! # Configuration
//!
//! - Bind address: Set via `BIND_ADDRESS` environment variable
//! - Default: `127.0.0.1:8080`
//!
//! # Features
//!
//! This template server demonstrates:
//! - **Method Registration**: Mapping method names to handler functions
//! - **Type Safety**: Using typed parameters and results
//! - **Configuration**: Reading settings from environment
//!
//! # Optional Features (Commented Out)
//!
//! Uncomment in the code to enable:
//! - **Logging**: `tracing_subscriber::fmt::init()`
//! - **Observability**: `.with_default_observability()`
//! - **Batch Processing**: `.batch_mode(BatchMode::Parallel)`
//! - **Middleware**: `.use_middleware(Arc::new(LoggingMiddleware::new()))`
//!
//! # Running
//!
//! ```bash
//! # Default (127.0.0.1:8080)
//! cargo run --bin server
//!
//! # Custom address
//! BIND_ADDRESS=0.0.0.0:9000 cargo run --bin server
//! ```
//!
//! # Testing
//!
//! Run the example client to test the server:
//! ```bash
//! cargo run --bin client
//! ```

use jrow_server::{from_typed_fn, ServerBuilder};
use my_jrow_app::{add_handler, echo_handler, status_handler};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging (optional)
    // tracing_subscriber::fmt::init();

    // Parse bind address from environment or use default
    let bind_addr: SocketAddr = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()?;

    println!("Starting JROW server on {}", bind_addr);

    // Build and configure the server
    let server = ServerBuilder::new()
        .bind(bind_addr)
        // Register RPC method handlers
        .handler("add", from_typed_fn(add_handler))
        .handler("echo", from_typed_fn(echo_handler))
        .handler("status", from_typed_fn(status_handler))
        // Optional: Enable observability
        // .with_default_observability()
        // Optional: Configure batch processing
        // .batch_mode(BatchMode::Parallel)
        // .max_batch_size(100)
        // Optional: Add middleware
        // .use_middleware(Arc::new(LoggingMiddleware::new()))
        .build()
        .await?;

    println!("Server ready! Available methods:");
    println!("  - add(a, b) -> sum");
    println!("  - echo(message) -> message");
    println!("  - status() -> {{status, uptime_seconds}}");

    // Run the server
    server.run().await?;

    Ok(())
}



