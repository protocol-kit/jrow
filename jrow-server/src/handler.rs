//! Handler traits and types for JSON-RPC methods
//!
//! This module defines the core abstraction for JSON-RPC method handlers. Handlers
//! are responsible for processing incoming JSON-RPC requests and producing results.
//!
//! # Handler Trait
//!
//! The `Handler` trait is the fundamental interface that all method implementations
//! must satisfy. It's designed to be:
//!
//! - **Async-compatible**: Returns a pinned future for non-blocking execution
//! - **Thread-safe**: Requires `Send + Sync` for multi-threaded server use
//! - **Type-erased**: Works with `serde_json::Value` for maximum flexibility
//!
//! # Creating Handlers
//!
//! There are several ways to create handlers:
//!
//! 1. **from_fn**: Wrap an async closure that works with raw JSON values
//! 2. **from_typed_fn**: Wrap an async closure with automatic type conversion
//! 3. **#[handler] macro**: Annotate a function to generate a handler (via jrow-macros)
//!
//! # Why Box<dyn Future>?
//!
//! Handlers return `HandlerResult` which is a type alias for a boxed, pinned future.
//! This is necessary because:
//! - Different handlers have different concrete future types
//! - We need a single type to store in the router's HashMap
//! - Boxing has minimal overhead compared to network I/O
//!
//! # Examples
//!
//! ```rust
//! use jrow_server::{Handler, from_fn, from_typed_fn};
//! use jrow_core::Result;
//! use serde::{Deserialize, Serialize};
//!
//! // Raw JSON handler
//! let handler1 = from_fn(|params| async move {
//!     Ok(serde_json::json!({"status": "ok"}))
//! });
//!
//! // Typed handler
//! #[derive(Deserialize)]
//! struct AddParams { a: i32, b: i32 }
//!
//! let handler2 = from_typed_fn(|params: AddParams| async move {
//!     Ok(params.a + params.b)
//! });
//! ```

use jrow_core::{Error, Result};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Result type for handler functions
///
/// This is a pinned, boxed future that resolves to a `Result<Value>`.
/// The boxing is necessary because different handlers return different
/// concrete future types, and we need a uniform type for storage.
///
/// # Why Pin?
///
/// Futures in Rust must be pinned before they can be polled. The `Pin`
/// ensures the future's memory location is stable, which is required
/// for self-referential types.
///
/// # Why Send?
///
/// The `Send` bound allows the future to be sent across threads,
/// which is essential for the multi-threaded Tokio runtime.
pub type HandlerResult = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;

/// Trait for JSON-RPC method handlers
///
/// This trait defines the interface that all JSON-RPC method implementations
/// must satisfy. It's designed to work with the router to dispatch requests
/// to the appropriate handler based on method name.
///
/// # Thread Safety
///
/// Handlers must be `Send + Sync` because:
/// - `Send`: The handler may be moved between threads
/// - `Sync`: Multiple threads may hold references to the same handler
///
/// This is safe because handlers should be stateless or use interior mutability.
///
/// # Implementation Notes
///
/// You typically don't implement this trait directly. Instead, use:
/// - `from_fn` for raw JSON value handlers
/// - `from_typed_fn` for type-safe handlers with automatic deserialization
/// - `#[handler]` macro for even more ergonomic handler definition
///
/// # Examples
///
/// ```rust
/// use jrow_server::{Handler, from_fn};
/// use jrow_core::Result;
///
/// let handler = from_fn(|params| async move {
///     // params is Option<serde_json::Value>
///     Ok(serde_json::json!({"echo": params}))
/// });
///
/// // Handler can now be registered with a router
/// ```
pub trait Handler: Send + Sync {
    /// Handle a JSON-RPC request and return a result
    ///
    /// This method is called by the server when a request arrives for
    /// the method this handler is registered to handle.
    ///
    /// # Arguments
    ///
    /// * `params` - Optional JSON value containing the request parameters.
    ///              `None` if the request had no params field.
    ///
    /// # Returns
    ///
    /// A future that resolves to:
    /// - `Ok(Value)`: Successful result to be sent back to the client
    /// - `Err(Error)`: Error to be converted to a JSON-RPC error response
    ///
    /// # Error Handling
    ///
    /// Errors returned from this method are automatically converted to
    /// JSON-RPC error responses with appropriate error codes:
    /// - `Error::InvalidParams` → -32602 (Invalid params)
    /// - `Error::MethodNotFound` → -32601 (Method not found)
    /// - `Error::Internal` → -32603 (Internal error)
    fn handle(&self, params: Option<Value>) -> HandlerResult;
}

/// Wrapper that adapts an async function into a Handler
///
/// This struct bridges the gap between regular async functions and the `Handler`
/// trait. It stores a function that takes optional JSON params and returns a
/// future producing a result.
///
/// # Type Parameters
///
/// * `F` - The function type (closure or function pointer)
/// * `Fut` - The future type returned by F
///
/// # Why This Wrapper?
///
/// We can't implement `Handler` directly for closures because of orphan rules
/// (can't implement external trait for external type). This wrapper provides
/// a type we own, allowing the implementation.
///
/// # Examples
///
/// ```rust
/// use jrow_server::from_fn;
/// use jrow_core::Result;
///
/// // AsyncHandler is internal, use from_fn instead
/// let handler = from_fn(|params| async move {
///     Ok(serde_json::json!({"received": params}))
/// });
/// ```
pub struct AsyncHandler<F, Fut>
where
    F: Fn(Option<Value>) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Value>> + Send + 'static,
{
    /// The wrapped async function
    func: F,
}

impl<F, Fut> AsyncHandler<F, Fut>
where
    F: Fn(Option<Value>) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Value>> + Send + 'static,
{
    /// Create a new async handler from a function
    ///
    /// # Arguments
    ///
    /// * `func` - An async function or closure that takes optional JSON params
    ///            and returns a future producing a Result<Value>
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jrow_server::from_fn;
    ///
    /// // AsyncHandler is internal, use from_fn instead
    /// let handler = from_fn(|_params| async {
    ///     Ok(serde_json::json!({"message": "hello"}))
    /// });
    /// ```
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F, Fut> Handler for AsyncHandler<F, Fut>
where
    F: Fn(Option<Value>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Value>> + Send + 'static,
{
    /// Implement Handler by calling the wrapped function and boxing the result
    ///
    /// This delegates to the wrapped function, then boxes and pins the returned
    /// future to satisfy the HandlerResult type.
    fn handle(&self, params: Option<Value>) -> HandlerResult {
        // Call the wrapped function to get a future, then box and pin it
        Box::pin((self.func)(params))
    }
}

/// Create a handler from an async function that works with raw JSON values
///
/// This is the simplest way to create a handler when you want to work directly
/// with `serde_json::Value`. The function receives the raw params and must
/// return a raw JSON value.
///
/// # Type Parameters
///
/// * `F` - The function/closure type
/// * `Fut` - The future type returned by the function
///
/// # Arguments
///
/// * `func` - An async function that takes `Option<Value>` and returns `Result<Value>`
///
/// # Returns
///
/// A boxed `Handler` ready to be registered with a router
///
/// # Examples
///
/// ```rust
/// use jrow_server::from_fn;
/// use jrow_core::Result;
///
/// let handler = from_fn(|params| async move {
///     match params {
///         Some(p) => Ok(serde_json::json!({"echo": p})),
///         None => Ok(serde_json::json!({"message": "no params"})),
///     }
/// });
/// ```
pub fn from_fn<F, Fut>(func: F) -> Box<dyn Handler>
where
    F: Fn(Option<Value>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Value>> + Send + 'static,
{
    Box::new(AsyncHandler::new(func))
}

/// Create a handler from an async function with automatic type conversion
///
/// This is the preferred way to create handlers when you want type safety.
/// It automatically handles:
/// - Deserializing JSON params into your parameter type
/// - Serializing your return value to JSON
/// - Converting deserialization errors to InvalidParams errors
///
/// # Type Parameters
///
/// * `P` - Parameter type (must implement Deserialize)
/// * `R` - Return type (must implement Serialize)
/// * `F` - Function/closure type
/// * `Fut` - Future type returned by the function
///
/// # Arguments
///
/// * `func` - An async function that takes `P` and returns `Result<R>`
///
/// # Returns
///
/// A boxed `Handler` with automatic type conversion
///
/// # Error Handling
///
/// - If params can't be deserialized to `P`: Returns `Error::InvalidParams`
/// - If result can't be serialized to JSON: Returns `Error::Serialization`
/// - Function errors are passed through unchanged
///
/// # Examples
///
/// ```rust
/// use jrow_server::from_typed_fn;
/// use jrow_core::Result;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Deserialize)]
/// struct AddParams {
///     a: i32,
///     b: i32,
/// }
///
/// #[derive(Serialize)]
/// struct AddResult {
///     sum: i32,
/// }
///
/// let handler = from_typed_fn(|params: AddParams| async move {
///     Ok(AddResult {
///         sum: params.a + params.b,
///     })
/// });
/// ```
///
/// # Why Arc?
///
/// The function is wrapped in `Arc` because we need to clone it into the
/// async block, but closures aren't `Clone`. `Arc` provides shared ownership
/// with minimal overhead.
pub fn from_typed_fn<P, R, F, Fut>(func: F) -> Box<dyn Handler>
where
    P: serde::de::DeserializeOwned + Send + 'static,
    R: serde::Serialize + Send + 'static,
    F: Fn(P) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R>> + Send + 'static,
{
    use std::sync::Arc;
    // Wrap function in Arc so we can clone it into the async block
    let func = Arc::new(func);

    from_fn(move |params: Option<Value>| {
        // Clone the Arc for this invocation
        let func = Arc::clone(&func);
        async move {
            // Deserialize params to the expected type P
            // If params is None, try to deserialize from null (works for unit type)
            let params: P = match params {
                Some(p) => {
                    serde_json::from_value(p).map_err(|e| Error::InvalidParams(e.to_string()))?
                }
                None => serde_json::from_value(Value::Null)
                    .map_err(|e| Error::InvalidParams(e.to_string()))?,
            };

            // Call the user's function with the deserialized params
            let result = func(params).await?;
            
            // Serialize the result back to JSON
            let value =
                serde_json::to_value(result).map_err(|e| Error::Serialization(e.to_string()))?;
            Ok(value)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct AddParams {
        a: i32,
        b: i32,
    }

    #[derive(Serialize, Deserialize)]
    struct AddResult {
        sum: i32,
    }

    #[tokio::test]
    async fn test_typed_handler() {
        let handler = from_typed_fn(|params: AddParams| async move {
            Ok(AddResult {
                sum: params.a + params.b,
            })
        });

        let params = serde_json::json!({"a": 5, "b": 3});
        let result = handler.handle(Some(params)).await.unwrap();

        let sum: AddResult = serde_json::from_value(result).unwrap();
        assert_eq!(sum.sum, 8);
    }
}
