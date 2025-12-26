//! Example demonstrating middleware usage

use jrow_server::{
    from_typed_fn, JrowServer, LoggingMiddleware, MetricsMiddleware, Middleware, MiddlewareAction,
    MiddlewareContext, SyncMiddleware,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Serialize)]
struct AddResult {
    sum: i32,
}

// Custom authentication middleware
struct AuthMiddleware;

impl SyncMiddleware for AuthMiddleware {
    fn pre_handle(
        &self,
        ctx: &mut MiddlewareContext,
    ) -> jrow_core::Result<MiddlewareAction> {
        // In a real app, check auth token from metadata
        println!("[Auth] Checking authentication for method: {}", ctx.method);
        
        // For demo, allow all requests
        Ok(MiddlewareAction::Continue)
    }

    fn post_handle(
        &self,
        _ctx: &mut MiddlewareContext,
        _result: &jrow_core::Result<serde_json::Value>,
    ) -> jrow_core::Result<()> {
        Ok(())
    }
}

// Custom rate limiting middleware
struct RateLimitMiddleware {
    max_requests: u32,
    current: Arc<std::sync::atomic::AtomicU32>,
}

impl RateLimitMiddleware {
    fn new(max_requests: u32) -> Self {
        Self {
            max_requests,
            current: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }
}

impl SyncMiddleware for RateLimitMiddleware {
    fn pre_handle(
        &self,
        ctx: &mut MiddlewareContext,
    ) -> jrow_core::Result<MiddlewareAction> {
        let count = self
            .current
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if count >= self.max_requests {
            println!("[RateLimit] Rate limit exceeded for conn_id: {}", ctx.conn_id);
            // Short-circuit and return error
            return Ok(MiddlewareAction::ShortCircuit(serde_json::json!({
                "error": "Rate limit exceeded"
            })));
        }
        
        println!("[RateLimit] Request {}/{} for conn_id: {}", count + 1, self.max_requests, ctx.conn_id);
        Ok(MiddlewareAction::Continue)
    }

    fn post_handle(
        &self,
        _ctx: &mut MiddlewareContext,
        _result: &jrow_core::Result<serde_json::Value>,
    ) -> jrow_core::Result<()> {
        // Decrement counter after request completes
        self.current
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting server with middleware...\n");

    // Create metrics middleware to track stats
    let metrics = Arc::new(MetricsMiddleware::new());
    let metrics_clone = metrics.clone();

    // Create server with multiple middleware
    let server = JrowServer::builder()
        .bind_str("127.0.0.1:9005")?
        // Middleware execute in order: Auth -> RateLimit -> Logging -> Metrics
        .use_sync_middleware(AuthMiddleware)
        .use_sync_middleware(RateLimitMiddleware::new(100))
        .use_sync_middleware(LoggingMiddleware::new())
        .use_middleware(metrics)
        .handler(
            "add",
            from_typed_fn(|params: AddParams| async move {
                Ok(AddResult {
                    sum: params.a + params.b,
                })
            }),
        )
        .handler(
            "multiply",
            from_typed_fn(|params: AddParams| async move {
                Ok(AddResult {
                    sum: params.a * params.b,
                })
            }),
        )
        .build()
        .await?;

    println!("Server listening on 127.0.0.1:9005");
    println!("Middleware chain:");
    println!("  1. AuthMiddleware - Authentication check");
    println!("  2. RateLimitMiddleware - Rate limiting (100 req/s)");
    println!("  3. LoggingMiddleware - Request/response logging");
    println!("  4. MetricsMiddleware - Performance metrics\n");

    // Spawn task to print metrics periodically
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            println!(
                "\n[Stats] Total requests processed: {}\n",
                metrics_clone.get_request_count()
            );
        }
    });

    server.run().await?;

    Ok(())
}



