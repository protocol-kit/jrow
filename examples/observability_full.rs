//! Full observability example
//!
//! Demonstrates comprehensive OpenTelemetry integration including:
//! - Distributed tracing across client and server
//! - Pub/sub with tracing
//! - Batch requests with metrics
//! - Reconnection with spans
//!
//! Run with: cargo run --example observability_full

use jrow_client::{BatchRequest, ClientBuilder};
use jrow_core::ObservabilityConfig;
use jrow_server::{from_typed_fn, ServerBuilder, TracingMiddleware};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
struct EchoParams {
    message: String,
}

#[derive(Serialize, Deserialize)]
struct EchoResult {
    echo: String,
}

async fn echo_handler(params: EchoParams) -> jrow_core::Result<EchoResult> {
    tracing::info!(message = %params.message, "Echo request received");
    Ok(EchoResult {
        echo: format!("Echo: {}", params.message),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== JROW Full Observability Example ===\n");

    // Start server with observability
    let server_config = ObservabilityConfig::new("jrow-full-server")
        .with_endpoint("http://localhost:4317")
        .with_log_level("info");

    let addr: std::net::SocketAddr = "127.0.0.1:9011".parse()?;
    let server = ServerBuilder::new()
        .bind(addr)
        .handler("echo", from_typed_fn(echo_handler))
        .use_middleware(Arc::new(TracingMiddleware::new()))
        .with_observability(server_config)
        .service_name("full-example-server")
        .build()
        .await?;

    let server = Arc::new(server);
    let server_clone = Arc::clone(&server);

    // Run server in background
    tokio::spawn(async move {
        if let Err(e) = server_clone.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Connect client (observability already initialized)
    let client = ClientBuilder::new("ws://127.0.0.1:9011")
        .service_name("full-example-client")
        .with_default_reconnect()
        .connect()
        .await?;

    println!("✓ Server and client started with observability\n");

    // 1. Individual requests with tracing
    println!("1. Individual Requests:");
    for i in 1..=3 {
        let result: EchoResult = client
            .request("echo", EchoParams {
                message: format!("Message {}", i),
            })
            .await?;
        println!("   {}", result.echo);
    }

    // 2. Batch requests with metrics
    println!("\n2. Batch Request:");
    let mut batch = BatchRequest::new();
    let id1 = batch.add_request("echo", EchoParams {
        message: "Batch 1".to_string(),
    });
    let id2 = batch.add_request("echo", EchoParams {
        message: "Batch 2".to_string(),
    });
    let id3 = batch.add_request("echo", EchoParams {
        message: "Batch 3".to_string(),
    });

    let batch_response = client.batch(batch).await?;
    for (i, id) in [&id1, &id2, &id3].iter().enumerate() {
        match batch_response.get::<EchoResult>(id) {
            Ok(echo) => println!("   Batch result {}: {}", i + 1, echo.echo),
            Err(e) => println!("   Batch result {}: Error - {:?}", i + 1, e),
        }
    }

    // 3. Pub/sub with tracing
    println!("\n3. Pub/Sub with Tracing:");
    
    client
        .subscribe("events", |_notification| {
            Box::pin(async move {
                tracing::info!(
                    topic = "events",
                    "Notification received"
                );
                println!("   Received event notification");
            })
        })
        .await?;

    // Publish some events
    server
        .publish("events", serde_json::json!({"type": "test", "data": "Hello"}))
        .await?;
    
    server
        .publish("events", serde_json::json!({"type": "test", "data": "World"}))
        .await?;

    // Wait for notifications to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("\n✓ All operations completed!");
    println!("\nView telemetry data:");
    println!("  - Traces: http://localhost:16686 (Jaeger UI)");
    println!("  - Metrics: Check your OTLP collector/backend");
    println!("  - Logs: Structured JSON logs in console");

    // Give time for telemetry to flush
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Cleanup
    jrow_core::shutdown_observability();

    Ok(())
}

