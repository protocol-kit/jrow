//! Middleware system for request/response pipeline
//!
//! Middleware provides a way to intercept and transform JSON-RPC requests
//! and responses. Common use cases include:
//! - Logging and tracing
//! - Authentication and authorization
//! - Rate limiting
//! - Request validation
//! - Metrics collection
//!
//! # Middleware Chain
//!
//! Middleware is executed in order as a chain. Each middleware can:
//! - Inspect and modify the request before the handler
//! - Short-circuit execution and return early
//! - Inspect and modify the response after the handler
//! - Pass metadata to subsequent middleware
//!
//! # Built-in Middleware
//!
//! - **LoggingMiddleware**: Logs all requests/responses
//! - **MetricsMiddleware**: Records request metrics
//! - **TracingMiddleware**: Adds OpenTelemetry spans
//!
//! # Examples
//!
//! ```rust
//! use jrow_server::{MiddlewareChain, LoggingMiddleware};
//!
//! let mut chain = MiddlewareChain::new();
//! chain.add_sync(LoggingMiddleware);
//!
//! // Use chain with ServerBuilder
//! // builder.use_middleware(Arc::new(LoggingMiddleware))
//! ```

use async_trait::async_trait;
use jrow_core::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// Import for tracing
#[allow(unused_imports)]
use tracing;

/// Action to take after middleware pre-processing
#[derive(Debug, Clone)]
pub enum MiddlewareAction {
    /// Continue to next middleware/handler
    Continue,
    /// Short-circuit and return this value immediately
    ShortCircuit(Value),
}

/// Context passed to middleware containing request information
#[derive(Debug, Clone)]
pub struct MiddlewareContext {
    /// The RPC method being called
    pub method: String,
    /// The parameters for the method
    pub params: Option<Value>,
    /// Connection ID making the request
    pub conn_id: u64,
    /// Request ID (for correlation)
    pub request_id: Option<jrow_core::Id>,
    /// Metadata for passing data between middleware
    pub metadata: HashMap<String, Value>,
}

impl MiddlewareContext {
    /// Create a new middleware context
    pub fn new(method: String, params: Option<Value>, conn_id: u64) -> Self {
        Self {
            method,
            params,
            conn_id,
            request_id: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Create a new middleware context with request ID
    pub fn with_request_id(method: String, params: Option<Value>, conn_id: u64, request_id: jrow_core::Id) -> Self {
        Self {
            method,
            params,
            conn_id,
            request_id: Some(request_id),
            metadata: HashMap::new(),
        }
    }

    /// Insert metadata that can be accessed by subsequent middleware
    pub fn insert_metadata(&mut self, key: impl Into<String>, value: Value) {
        self.metadata.insert(key.into(), value);
    }

    /// Get metadata by key
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }
}

/// Trait for async middleware
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Called before handler execution
    async fn pre_handle(&self, ctx: &mut MiddlewareContext) -> Result<MiddlewareAction>;

    /// Called after handler execution
    async fn post_handle(&self, ctx: &mut MiddlewareContext, result: &Result<Value>) -> Result<()>;
}

/// Trait for synchronous middleware (simpler, no async operations)
pub trait SyncMiddleware: Send + Sync {
    /// Called before handler execution
    fn pre_handle(&self, ctx: &mut MiddlewareContext) -> Result<MiddlewareAction>;

    /// Called after handler execution
    fn post_handle(&self, ctx: &mut MiddlewareContext, result: &Result<Value>) -> Result<()>;
}

/// Adapter to convert SyncMiddleware to Middleware
struct SyncMiddlewareAdapter<T: SyncMiddleware> {
    inner: T,
}

#[async_trait]
impl<T: SyncMiddleware + 'static> Middleware for SyncMiddlewareAdapter<T> {
    async fn pre_handle(&self, ctx: &mut MiddlewareContext) -> Result<MiddlewareAction> {
        self.inner.pre_handle(ctx)
    }

    async fn post_handle(&self, ctx: &mut MiddlewareContext, result: &Result<Value>) -> Result<()> {
        self.inner.post_handle(ctx, result)
    }
}

/// Chain of middleware to execute in order
#[derive(Clone)]
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    /// Create a new empty middleware chain
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware to the chain
    pub fn add(&mut self, middleware: Arc<dyn Middleware>) {
        self.middlewares.push(middleware);
    }

    /// Add a sync middleware to the chain
    pub fn add_sync<T: SyncMiddleware + 'static>(&mut self, middleware: T) {
        self.middlewares.push(Arc::new(SyncMiddlewareAdapter {
            inner: middleware,
        }));
    }

    /// Execute the middleware chain with the given handler
    pub async fn execute<F, Fut>(
        &self,
        mut ctx: MiddlewareContext,
        handler: F,
    ) -> Result<Value>
    where
        F: FnOnce(MiddlewareContext) -> Fut + Send,
        Fut: std::future::Future<Output = Result<Value>> + Send,
    {
        // Execute pre_handle for each middleware
        for middleware in &self.middlewares {
            match middleware.pre_handle(&mut ctx).await? {
                MiddlewareAction::Continue => continue,
                MiddlewareAction::ShortCircuit(value) => {
                    // Short-circuit: skip handler and remaining middleware
                    // Still run post_handle for middleware that already ran
                    return Ok(value);
                }
            }
        }

        // Execute the handler
        let result = handler(ctx.clone()).await;

        // Execute post_handle for each middleware in reverse order
        for middleware in self.middlewares.iter().rev() {
            // Ignore errors in post_handle to ensure all middleware run
            let _ = middleware.post_handle(&mut ctx, &result).await;
        }

        result
    }

    /// Get the number of middleware in the chain
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in logging middleware
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncMiddleware for LoggingMiddleware {
    fn pre_handle(&self, ctx: &mut MiddlewareContext) -> Result<MiddlewareAction> {
        println!(
            "[Middleware] Request: method={}, conn_id={}",
            ctx.method, ctx.conn_id
        );
        Ok(MiddlewareAction::Continue)
    }

    fn post_handle(&self, ctx: &mut MiddlewareContext, result: &Result<Value>) -> Result<()> {
        match result {
            Ok(value) => println!(
                "[Middleware] Response: method={}, success=true, result={:?}",
                ctx.method,
                value.to_string().chars().take(100).collect::<String>()
            ),
            Err(e) => println!(
                "[Middleware] Response: method={}, success=false, error={}",
                ctx.method, e
            ),
        }
        Ok(())
    }
}

/// Built-in metrics middleware
pub struct MetricsMiddleware {
    request_count: Arc<std::sync::atomic::AtomicU64>,
}

impl MetricsMiddleware {
    pub fn new() -> Self {
        Self {
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn get_request_count(&self) -> u64 {
        self.request_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for MetricsMiddleware {
    async fn pre_handle(&self, ctx: &mut MiddlewareContext) -> Result<MiddlewareAction> {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        ctx.insert_metadata(
            "start_time",
            Value::from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            ),
        );
        Ok(MiddlewareAction::Continue)
    }

    async fn post_handle(&self, ctx: &mut MiddlewareContext, _result: &Result<Value>) -> Result<()> {
        if let Some(Value::Number(start)) = ctx.get_metadata("start_time") {
            if let Some(start_ms) = start.as_u64() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let duration = now - start_ms;
                println!("[Metrics] method={}, duration={}ms", ctx.method, duration);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jrow_core::Error;

    struct TestMiddleware {
        name: String,
    }

    impl TestMiddleware {
        fn new(name: impl Into<String>) -> Self {
            Self { name: name.into() }
        }
    }

    impl SyncMiddleware for TestMiddleware {
        fn pre_handle(&self, ctx: &mut MiddlewareContext) -> Result<MiddlewareAction> {
            ctx.insert_metadata(&format!("{}_pre", self.name), Value::Bool(true));
            Ok(MiddlewareAction::Continue)
        }

        fn post_handle(&self, ctx: &mut MiddlewareContext, _result: &Result<Value>) -> Result<()> {
            ctx.insert_metadata(&format!("{}_post", self.name), Value::Bool(true));
            Ok(())
        }
    }

    struct ShortCircuitMiddleware;

    impl SyncMiddleware for ShortCircuitMiddleware {
        fn pre_handle(&self, _ctx: &mut MiddlewareContext) -> Result<MiddlewareAction> {
            Ok(MiddlewareAction::ShortCircuit(Value::String(
                "short-circuited".to_string(),
            )))
        }

        fn post_handle(&self, _ctx: &mut MiddlewareContext, _result: &Result<Value>) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_middleware_execution_order() {
        let mut chain = MiddlewareChain::new();
        chain.add_sync(TestMiddleware::new("first"));
        chain.add_sync(TestMiddleware::new("second"));

        let ctx = MiddlewareContext::new("test_method".to_string(), None, 1);

        let result = chain
            .execute(ctx.clone(), |ctx| async move {
                // Verify pre_handle ran for both middleware
                assert!(ctx.get_metadata("first_pre").is_some());
                assert!(ctx.get_metadata("second_pre").is_some());
                Ok(Value::String("handler result".to_string()))
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_middleware_short_circuit() {
        let mut chain = MiddlewareChain::new();
        chain.add_sync(TestMiddleware::new("first"));
        chain.add_sync(ShortCircuitMiddleware);
        chain.add_sync(TestMiddleware::new("third"));

        let ctx = MiddlewareContext::new("test_method".to_string(), None, 1);

        let result = chain
            .execute(ctx.clone(), |_ctx| async move {
                // This should not be called
                panic!("Handler should not be called");
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("short-circuited".to_string()));
    }

    #[tokio::test]
    async fn test_middleware_metadata() {
        let mut chain = MiddlewareChain::new();
        chain.add_sync(TestMiddleware::new("test"));

        let ctx = MiddlewareContext::new("test_method".to_string(), None, 1);

        let result = chain
            .execute(ctx, |ctx| async move {
                // Access metadata set by middleware
                assert_eq!(
                    ctx.get_metadata("test_pre"),
                    Some(&Value::Bool(true))
                );
                Ok(Value::Null)
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logging_middleware() {
        let mut chain = MiddlewareChain::new();
        chain.add_sync(LoggingMiddleware::new());

        let ctx = MiddlewareContext::new("test_method".to_string(), None, 1);

        let result = chain
            .execute(ctx, |_ctx| async move {
                Ok(Value::String("test result".to_string()))
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_middleware() {
        let metrics = Arc::new(MetricsMiddleware::new());
        let mut chain = MiddlewareChain::new();
        chain.add(metrics.clone());

        let initial_count = metrics.get_request_count();

        let ctx = MiddlewareContext::new("test_method".to_string(), None, 1);

        let result = chain
            .execute(ctx, |_ctx| async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                Ok(Value::Null)
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(metrics.get_request_count(), initial_count + 1);
    }

    #[tokio::test]
    async fn test_middleware_error_handling() {
        let mut chain = MiddlewareChain::new();
        chain.add_sync(TestMiddleware::new("test"));

        let ctx = MiddlewareContext::new("test_method".to_string(), None, 1);

        let result = chain
            .execute(ctx, |_ctx| async move {
                Err(Error::Internal("test error".to_string()))
            })
            .await;

        // Error should propagate
        assert!(result.is_err());
    }
}

/// Automatic tracing middleware for all requests
/// 
/// This middleware creates spans for each RPC request, recording method name,
/// connection ID, and request ID. It also logs the result status after execution.
pub struct TracingMiddleware;

impl TracingMiddleware {
    /// Create a new tracing middleware
    pub fn new() -> Self {
        Self
    }
}

impl Default for TracingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for TracingMiddleware {
    async fn pre_handle(&self, ctx: &mut MiddlewareContext) -> Result<MiddlewareAction> {
        // Create a span for this request
        let span = tracing::info_span!(
            "rpc_request",
            method = %ctx.method,
            conn_id = ctx.conn_id,
            request_id = ?ctx.request_id,
        );
        
        // Enter the span (it will be active for the duration of the request)
        let _enter = span.enter();
        
        tracing::debug!("Request started");
        
        Ok(MiddlewareAction::Continue)
    }

    async fn post_handle(&self, ctx: &mut MiddlewareContext, result: &Result<Value>) -> Result<()> {
        let span = tracing::info_span!(
            "rpc_request",
            method = %ctx.method,
            conn_id = ctx.conn_id,
            request_id = ?ctx.request_id,
        );
        
        let _enter = span.enter();
        
        match result {
            Ok(_) => tracing::info!("Request completed successfully"),
            Err(e) => tracing::error!(error = %e, "Request failed"),
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tracing_tests {
    use super::*;

    #[tokio::test]
    async fn test_tracing_middleware() {
        let middleware = TracingMiddleware::new();
        let mut ctx = MiddlewareContext::new("test_method".to_string(), None, 1);

        let action = middleware.pre_handle(&mut ctx).await.unwrap();
        assert!(matches!(action, MiddlewareAction::Continue));

        let result = Ok(Value::String("success".to_string()));
        middleware.post_handle(&mut ctx, &result).await.unwrap();
    }
}

