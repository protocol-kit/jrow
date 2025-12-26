//! Procedural macros for jrow JSON-RPC toolkit
//!
//! This crate provides derive macros and attribute macros that reduce boilerplate
//! when building JSON-RPC servers and clients. The macros handle type conversions,
//! error handling, and trait implementations automatically.
//!
//! # Available Macros
//!
//! ## `#[handler]` - JSON-RPC Handler Function
//!
//! Transforms a regular async function into a JSON-RPC handler with automatic:
//! - Parameter deserialization from JSON-RPC params
//! - Return value serialization to JSON-RPC result
//! - Error mapping to JSON-RPC errors
//!
//! # How It Works
//!
//! The `#[handler]` macro performs compile-time code generation:
//!
//! 1. Parses the function signature to extract parameter and return types
//! 2. Wraps the function body in an inner async function
//! 3. Generates a factory function that returns a `Box<dyn Handler>`
//! 4. Uses `from_typed_fn` to create the handler with type conversion logic
//!
//! This means you write normal Rust functions with type-safe parameters, and
//! the macro generates all the JSON-RPC protocol handling automatically.
//!
//! # Benefits Over Manual Implementation
//!
//! Without macros, you'd write:
//!
//! ```ignore
//! pub fn add() -> Box<dyn Handler> {
//!     from_typed_fn(|params: AddParams| async move {
//!         Ok(params.a + params.b)
//!     })
//! }
//! ```
//!
//! With macros, you write:
//!
//! ```ignore
//! #[jrow::handler]
//! async fn add(params: AddParams) -> Result<i32, Error> {
//!     Ok(params.a + params.b)
//! }
//! ```
//!
//! The macro version is:
//! - **More readable**: Looks like a normal function
//! - **Less error-prone**: No manual wrapping or closure syntax
//! - **Type-safe**: Compiler catches parameter/return type mismatches
//!
//! # Examples
//!
//! ```ignore
//! use serde::{Deserialize, Serialize};
//! use jrow_core::Result;
//!
//! #[derive(Deserialize)]
//! struct AddParams {
//!     a: i32,
//!     b: i32,
//! }
//!
//! // This generates a function that returns Box<dyn Handler>
//! #[jrow::handler]
//! async fn add(params: AddParams) -> Result<i32> {
//!     Ok(params.a + params.b)
//! }
//!
//! // Use it in a router
//! let router = Router::new()
//!     .route("add", add());
//! ```

mod handler;

use proc_macro::TokenStream;

/// Attribute macro for defining JSON-RPC handlers
///
/// This macro transforms an async function into a JSON-RPC handler with automatic
/// parameter deserialization and result serialization. It generates a factory function
/// that returns `Box<dyn Handler>`, ready to be registered with a router.
///
/// # Generated Code
///
/// The macro converts:
///
/// ```ignore
/// #[handler]
/// async fn my_method(params: MyParams) -> Result<MyResult> {
///     // ... implementation ...
/// }
/// ```
///
/// Into approximately:
///
/// ```ignore
/// fn my_method() -> Box<dyn Handler> {
///     async fn inner_handler(params: MyParams) -> Result<MyResult> {
///         // ... implementation ...
///     }
///     from_typed_fn(inner_handler)
/// }
/// ```
///
/// # Parameter Types
///
/// The parameter type must implement `serde::Deserialize`. Common patterns:
///
/// - **Struct params**: `params: MyParams` for object parameters
/// - **Unit params**: `params: ()` for methods with no parameters
/// - **No params**: Omit the parameter entirely
///
/// # Return Types
///
/// The return type must:
/// - Be a `Result<T, E>` where `T: Serialize` and `E` converts to `jrow_core::Error`
/// - Or implement `Serialize` directly (though Result is recommended)
///
/// # Attributes and Visibility
///
/// The macro preserves:
/// - Function visibility (`pub`, `pub(crate)`, etc.)
/// - Doc comments and other attributes
///
/// # Examples
///
/// ## Simple handler with struct params
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct AddParams {
///     a: i32,
///     b: i32,
/// }
///
/// #[handler]
/// async fn add(params: AddParams) -> Result<i32> {
///     Ok(params.a + params.b)
/// }
/// ```
///
/// ## Handler with no parameters
///
/// ```ignore
/// #[handler]
/// async fn get_server_time() -> Result<String> {
///     Ok(chrono::Utc::now().to_rfc3339())
/// }
/// ```
///
/// ## Handler with complex return type
///
/// ```ignore
/// #[derive(Serialize)]
/// struct Status {
///     version: String,
///     uptime: u64,
/// }
///
/// #[handler]
/// async fn get_status(params: ()) -> Result<Status> {
///     Ok(Status {
///         version: env!("CARGO_PKG_VERSION").into(),
///         uptime: get_uptime(),
///     })
/// }
/// ```
///
/// # Error Handling
///
/// Errors are automatically converted to JSON-RPC errors:
/// - Return `Err(jrow_core::Error::InvalidParams(...))` for parameter validation errors
/// - Return `Err(jrow_core::Error::Internal(...))` for unexpected errors
/// - Custom error types can implement `Into<jrow_core::Error>`
///
/// # Limitations
///
/// - The macro only works with async functions
/// - Functions must have at most one parameter
/// - Cannot use `self` (this is for free functions, not methods)
#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    handler::handler_impl(item)
}


