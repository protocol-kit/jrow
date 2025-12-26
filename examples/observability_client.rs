//! Observability client example
//!
//! Demonstrates OpenTelemetry integration on the client side with distributed tracing.
//!
//! Run with: cargo run --example observability_client

use jrow_client::ClientBuilder;
use jrow_core::ObservabilityConfig;
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
struct MultiplyParams {
    a: i32,
    b: i32,
}

#[derive(Deserialize)]
struct MultiplyResult {
    product: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure observability
    let otel_config = ObservabilityConfig::new("jrow-observability-client")
        .with_endpoint("http://localhost:4317")
        .with_log_level("debug");

    // Connect with observability enabled
    let client = ClientBuilder::new("ws://127.0.0.1:9010")
        .with_observability(otel_config)
        .service_name("observability-client")
        .connect()
        .await?;

    println!("Connected to observability server");
    println!("Sending requests with distributed tracing...\n");

    // Send multiple requests to generate traces
    for i in 1..=5 {
        tracing::info!(iteration = i, "Starting iteration");

        // Add operation
        let add_result: AddResult = client
            .request("add", AddParams { a: i * 10, b: i * 5 })
            .await?;
        println!("Iteration {}: {} + {} = {}", i, i * 10, i * 5, add_result.sum);

        // Multiply operation
        let multiply_result: MultiplyResult = client
            .request("multiply", MultiplyParams { a: i * 2, b: i * 3 })
            .await?;
        println!(
            "Iteration {}: {} * {} = {}",
            i,
            i * 2,
            i * 3,
            multiply_result.product
        );

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    println!("\nAll requests completed!");
    println!("View distributed traces at http://localhost:16686 (Jaeger UI)");
    println!("Search for service: observability-client or observability-server");

    // Give time for telemetry to flush
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Cleanup
    jrow_core::shutdown_observability();

    Ok(())
}



