//! Observability server example
//!
//! Demonstrates OpenTelemetry integration on the server side.
//!
//! Run with: cargo run --example observability_server

use jrow_core::ObservabilityConfig;
use jrow_server::{from_typed_fn, ServerBuilder, TracingMiddleware};
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

async fn add_handler(params: AddParams) -> jrow_core::Result<AddResult> {
    tracing::info!(a = params.a, b = params.b, "Adding numbers");
    
    // Simulate some work
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    
    let sum = params.a + params.b;
    tracing::debug!(sum = sum, "Calculation complete");
    
    Ok(AddResult { sum })
}

#[derive(Deserialize)]
struct MultiplyParams {
    a: i32,
    b: i32,
}

#[derive(Serialize)]
struct MultiplyResult {
    product: i32,
}

async fn multiply_handler(params: MultiplyParams) -> jrow_core::Result<MultiplyResult> {
    tracing::info!(a = params.a, b = params.b, "Multiplying numbers");
    
    // Simulate some work
    tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    
    let product = params.a * params.b;
    tracing::debug!(product = product, "Calculation complete");
    
    Ok(MultiplyResult { product })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure observability
    let otel_config = ObservabilityConfig::new("jrow-observability-server")
        .with_endpoint("http://localhost:4317")
        .with_log_level("debug");

    // Build server with observability enabled
    let addr: std::net::SocketAddr = "127.0.0.1:9010".parse()?;
    let server = ServerBuilder::new()
        .bind(addr)
        .handler("add", from_typed_fn(add_handler))
        .handler("multiply", from_typed_fn(multiply_handler))
        .use_middleware(std::sync::Arc::new(TracingMiddleware::new()))
        .with_observability(otel_config)
        .service_name("observability-server")
        .build()
        .await?;

    println!("Observability server running on 127.0.0.1:9010");
    println!("Traces and metrics available at http://localhost:4317");
    println!("View traces at http://localhost:16686 (Jaeger UI)");
    println!();
    println!("Try: cargo run --example observability_client");

    server.run().await?;

    // Cleanup
    jrow_core::shutdown_observability();

    Ok(())
}

