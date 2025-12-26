//! Router for JSON-RPC method routing
//!
//! The router dispatches incoming JSON-RPC requests to their corresponding
//! handler functions based on the method name. It also manages middleware
//! that can intercept and transform requests/responses.
//!
//! # Core Responsibilities
//!
//! - **Method registration**: Map method names to handler implementations
//! - **Request routing**: Direct requests to the appropriate handler
//! - **Middleware execution**: Run middleware chain before/after handlers
//! - **Error handling**: Convert handler errors to JSON-RPC error responses
//!
//! # Thread Safety
//!
//! Routers are cheaply cloneable (`Arc`-based) and thread-safe, allowing
//! them to be shared across connection tasks without synchronization overhead.
//!
//! # Examples
//!
//! ```rust
//! use jrow_server::{Router, from_fn};
//!
//! let mut router = Router::new();
//!
//! // Register handlers
//! router.register("ping", from_fn(|_| async {
//!     Ok(serde_json::json!({"pong": true}))
//! }));
//!
//! router.register("echo", from_fn(|params| async move {
//!     Ok(params.unwrap_or_default())
//! }));
//! ```

use crate::handler::Handler;
use crate::middleware::{MiddlewareChain, MiddlewareContext};
use jrow_core::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;

/// Router for JSON-RPC methods
///
/// Maps method names to handler functions and manages middleware execution.
/// Routers are cloneable and thread-safe, stored in an `Arc` internally.
///
/// # Design
///
/// The router uses a HashMap for O(1) method lookup. Handlers are wrapped
/// in `Arc` so they can be shared across threads without cloning the
/// actual handler logic.
#[derive(Clone)]
pub struct Router {
    /// Map of method names to their handler implementations
    handlers: Arc<HashMap<String, Arc<dyn Handler>>>,
    /// Middleware chain for request/response processing
    middleware_chain: MiddlewareChain,
}

impl Router {
    /// Create a new empty router
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(HashMap::new()),
            middleware_chain: MiddlewareChain::new(),
        }
    }

    /// Create a router with middleware
    pub fn with_middleware(middleware_chain: MiddlewareChain) -> Self {
        Self {
            handlers: Arc::new(HashMap::new()),
            middleware_chain,
        }
    }

    /// Register a handler for a method
    pub fn register(&mut self, method: impl Into<String>, handler: Box<dyn Handler>) {
        let handlers = Arc::make_mut(&mut self.handlers);
        handlers.insert(method.into(), Arc::from(handler));
    }

    /// Set the middleware chain for this router
    pub fn set_middleware(&mut self, middleware_chain: MiddlewareChain) {
        self.middleware_chain = middleware_chain;
    }

    /// Get a handler for a method
    pub fn get(&self, method: &str) -> Option<Arc<dyn Handler>> {
        self.handlers.get(method).cloned()
    }

    /// Check if a method is registered
    pub fn has_method(&self, method: &str) -> bool {
        self.handlers.contains_key(method)
    }

    /// Get all registered method names
    pub fn methods(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    /// Route a method call to the appropriate handler
    pub async fn route(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        self.route_with_conn_id(method, params, 0).await
    }

    /// Route a method call with connection ID (for middleware)
    pub async fn route_with_conn_id(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        conn_id: u64,
    ) -> Result<serde_json::Value> {
        let handler = self
            .get(method)
            .ok_or_else(|| Error::MethodNotFound(method.to_string()))?;

        // If no middleware, execute handler directly
        if self.middleware_chain.is_empty() {
            return handler.handle(params).await;
        }

        // Create middleware context
        let ctx = MiddlewareContext::new(method.to_string(), params.clone(), conn_id);

        // Execute middleware chain with handler
        self.middleware_chain
            .execute(ctx, |ctx| async move {
                handler.handle(ctx.params).await
            })
            .await
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a router
pub struct RouterBuilder {
    router: Router,
}

impl RouterBuilder {
    /// Create a new router builder
    pub fn new() -> Self {
        Self {
            router: Router::new(),
        }
    }

    /// Create a router builder with middleware
    pub fn with_middleware(middleware_chain: MiddlewareChain) -> Self {
        Self {
            router: Router::with_middleware(middleware_chain),
        }
    }

    /// Add a handler for a method
    pub fn handler(mut self, method: impl Into<String>, handler: Box<dyn Handler>) -> Self {
        self.router.register(method, handler);
        self
    }

    /// Build the router
    pub fn build(self) -> Router {
        self.router
    }
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::from_fn;

    #[tokio::test]
    async fn test_router_basic() {
        let mut router = Router::new();

        let handler = from_fn(|_| async { Ok(serde_json::json!({"status": "ok"})) });
        router.register("test", handler);

        assert!(router.has_method("test"));
        assert!(!router.has_method("unknown"));

        let result = router.route("test", None).await.unwrap();
        assert_eq!(result, serde_json::json!({"status": "ok"}));
    }

    #[tokio::test]
    async fn test_router_method_not_found() {
        let router = Router::new();
        let result = router.route("unknown", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_router_builder() {
        let handler = from_fn(|_| async { Ok(serde_json::json!(42)) });

        let router = RouterBuilder::new().handler("method1", handler).build();

        assert!(router.has_method("method1"));
        let result = router.route("method1", None).await.unwrap();
        assert_eq!(result, serde_json::json!(42));
    }
}
