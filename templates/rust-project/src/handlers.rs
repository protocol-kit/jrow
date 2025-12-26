//! RPC method handlers
//!
//! This module implements the business logic for each JSON-RPC method.
//! Handlers are async functions that:
//! - Take typed parameters (from `types.rs`)
//! - Return `Result<T>` where T implements Serialize
//! - Can perform async operations (database, network, etc.)
//!
//! # Handler Pattern
//!
//! Each handler follows this pattern:
//! ```rust
//! use jrow_core::Result;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! struct MyParams { /* ... */ }
//!
//! #[derive(Serialize)]
//! struct MyResult { /* ... */ }
//!
//! async fn my_handler(params: MyParams) -> Result<MyResult> {
//!     // ... business logic ...
//!     Ok(MyResult { /* ... */ })
//! }
//! ```
//!
//! # Error Handling
//!
//! Return `jrow_core::Error` variants for different error cases:
//! - `Error::InvalidParams`: Parameter validation failed
//! - `Error::Internal`: Unexpected errors
//! - Custom errors can be mapped to appropriate variants
//!
//! # Registration
//!
//! Handlers are registered in `bin/server.rs` using:
//! ```rust,ignore
//! .handler("method_name", from_typed_fn(my_handler))
//! ```

use crate::types::*;
use jrow_core::Result;
use std::time::Instant;

lazy_static::lazy_static! {
    static ref SERVER_START_TIME: Instant = Instant::now();
}

/// Add two numbers
pub async fn add_handler(params: AddParams) -> Result<AddResult> {
    Ok(AddResult {
        sum: params.a + params.b,
    })
}

/// Echo a message back
pub async fn echo_handler(params: EchoParams) -> Result<EchoResult> {
    Ok(EchoResult {
        message: params.message,
    })
}

/// Get server status
pub async fn status_handler(_params: StatusParams) -> Result<StatusResult> {
    Ok(StatusResult {
        status: "running".to_string(),
        uptime_seconds: SERVER_START_TIME.elapsed().as_secs(),
    })
}



