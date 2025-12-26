//! Batch request example demonstrating parallel and sequential processing

use jrow_client::{BatchRequest, JrowClient};
use jrow_server::{from_typed_fn, BatchMode, JrowServer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Deserialize, Serialize)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Deserialize, Serialize)]
struct MultiplyParams {
    a: i32,
    b: i32,
}

#[derive(Deserialize, Serialize)]
struct LogParams {
    message: String,
}

/// Start the server
async fn start_server(
    mode: BatchMode,
    port: u16,
) -> Result<Arc<JrowServer>, Box<dyn std::error::Error>> {
    let add_handler = from_typed_fn(|params: AddParams| async move {
        sleep(Duration::from_millis(100)).await; // Simulate work
        Ok(params.a + params.b)
    });

    let multiply_handler = from_typed_fn(|params: MultiplyParams| async move {
        sleep(Duration::from_millis(100)).await; // Simulate work
        Ok(params.a * params.b)
    });

    let log_handler = from_typed_fn(|params: LogParams| async move {
        println!("[SERVER] Log: {}", params.message);
        Ok(serde_json::json!({"logged": true}))
    });

    let echo_handler = from_typed_fn(|params: serde_json::Value| async move { Ok(params) });

    let server = Arc::new(
        JrowServer::builder()
            .bind_str(&format!("127.0.0.1:{}", port))?
            .batch_mode(mode)
            .handler("add", add_handler)
            .handler("multiply", multiply_handler)
            .handler("log", log_handler)
            .handler("echo", echo_handler)
            .build()
            .await?,
    );

    println!(
        "[SERVER] Started on ws://127.0.0.1:{} (mode: {:?})",
        port, mode
    );

    Ok(server)
}

/// Test batch requests
async fn test_batch(port: u16, mode_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing {} Mode ===", mode_name);

    sleep(Duration::from_millis(200)).await; // Wait for server

    let client = JrowClient::connect(&format!("ws://127.0.0.1:{}", port)).await?;
    println!("[CLIENT] Connected");

    // Build a batch request
    let mut batch = BatchRequest::new();

    let id1 = batch.add_request("add", AddParams { a: 10, b: 20 });
    let id2 = batch.add_request("add", AddParams { a: 5, b: 15 });
    let id3 = batch.add_request("multiply", MultiplyParams { a: 3, b: 4 });
    let id4 = batch.add_request("multiply", MultiplyParams { a: 7, b: 8 });
    let id5 = batch.add_request("echo", serde_json::json!({"test": "data"}));

    // Add a notification (no response)
    batch.add_notification(
        "log",
        LogParams {
            message: format!("Batch request with {} items", batch.len()),
        },
    );

    println!(
        "[CLIENT] Sending batch with {} items ({} requests, {} notifications)",
        batch.len(),
        batch.request_ids().len(),
        batch.len() - batch.request_ids().len()
    );

    // Send batch and measure time
    let start = Instant::now();
    let responses = client.batch(batch).await?;
    let elapsed = start.elapsed();

    println!(
        "[CLIENT] Received {} responses in {:?}",
        responses.len(),
        elapsed
    );

    // Extract results
    let result1: i32 = responses.get(&id1)?;
    let result2: i32 = responses.get(&id2)?;
    let result3: i32 = responses.get(&id3)?;
    let result4: i32 = responses.get(&id4)?;
    let result5: serde_json::Value = responses.get(&id5)?;

    println!("[CLIENT] Results:");
    println!("  add(10, 20) = {}", result1);
    println!("  add(5, 15) = {}", result2);
    println!("  multiply(3, 4) = {}", result3);
    println!("  multiply(7, 8) = {}", result4);
    println!("  echo(...) = {}", result5);

    // Verify
    assert_eq!(result1, 30);
    assert_eq!(result2, 20);
    assert_eq!(result3, 12);
    assert_eq!(result4, 56);

    println!("[CLIENT] All results correct!");

    // Test error handling
    println!("\n[CLIENT] Testing error handling...");
    let mut error_batch = BatchRequest::new();
    let good_id = error_batch.add_request("add", AddParams { a: 1, b: 2 });
    let bad_id = error_batch.add_request("nonexistent", serde_json::json!({}));

    let error_responses = client.batch(error_batch).await?;

    // Check that we got both responses
    assert!(error_responses.has_response(&good_id));
    assert!(error_responses.has_response(&bad_id));

    // Good request should succeed
    let good_result: Result<i32, _> = error_responses.get(&good_id);
    assert!(good_result.is_ok());
    println!("[CLIENT] Good request succeeded: {}", good_result.unwrap());

    // Bad request should fail
    let bad_result: Result<serde_json::Value, _> = error_responses.get(&bad_id);
    assert!(bad_result.is_err());
    println!(
        "[CLIENT] Bad request failed as expected: {}",
        bad_result.unwrap_err()
    );

    println!("[CLIENT] Error handling test passed!");

    Ok(())
}

/// Compare batch vs individual requests
async fn compare_performance(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Performance Comparison ===");

    let client = JrowClient::connect(&format!("ws://127.0.0.1:{}", port)).await?;

    // Individual requests
    println!("[PERF] Sending 5 individual requests...");
    let start = Instant::now();
    for i in 0..5 {
        let _result: i32 = client.request("add", AddParams { a: i, b: i + 1 }).await?;
    }
    let individual_time = start.elapsed();
    println!("[PERF] Individual requests took: {:?}", individual_time);

    // Batch request
    println!("[PERF] Sending 5 requests in a batch...");
    let mut batch = BatchRequest::new();
    for i in 0..5 {
        batch.add_request("add", AddParams { a: i, b: i + 1 });
    }
    let start = Instant::now();
    let _responses = client.batch(batch).await?;
    let batch_time = start.elapsed();
    println!("[PERF] Batch request took: {:?}", batch_time);

    let speedup = individual_time.as_millis() as f64 / batch_time.as_millis() as f64;
    println!("[PERF] Speedup: {:.2}x faster", speedup);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== JSON-RPC Batch Request Example ===\n");

    // Test parallel mode
    let parallel_server = start_server(BatchMode::Parallel, 9001).await?;
    let parallel_handle = tokio::spawn({
        let server = Arc::clone(&parallel_server);
        async move {
            let _ = server.run().await;
        }
    });

    test_batch(9001, "Parallel").await?;
    compare_performance(9001).await?;

    // Test sequential mode
    let sequential_server = start_server(BatchMode::Sequential, 9002).await?;
    let sequential_handle = tokio::spawn({
        let server = Arc::clone(&sequential_server);
        async move {
            let _ = server.run().await;
        }
    });

    test_batch(9002, "Sequential").await?;

    println!("\n=== All Tests Passed! ===");

    // Cleanup
    parallel_handle.abort();
    sequential_handle.abort();

    sleep(Duration::from_millis(100)).await;

    Ok(())
}


