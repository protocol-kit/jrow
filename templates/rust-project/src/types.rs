//! Request and response types for RPC methods
//!
//! This module defines the data structures for JSON-RPC method parameters
//! and results. Each RPC method typically has:
//! - A `*Params` struct implementing `Deserialize` for parameters
//! - A `*Result` struct implementing `Serialize` for results
//!
//! # Design Pattern
//!
//! Using dedicated types (instead of raw JSON) provides:
//! - **Type safety**: Compile-time validation of parameter structure
//! - **Documentation**: Types serve as API documentation
//! - **IDE support**: Auto-completion and type hints
//! - **Validation**: Automatic validation via serde
//!
//! # Examples
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//!
//! // Parameters for "calculate" method
//! #[derive(Deserialize)]
//! struct CalculateParams {
//!     operation: String,
//!     a: f64,
//!     b: f64,
//! }
//!
//! // Result from "calculate" method
//! #[derive(Serialize)]
//! struct CalculateResult {
//!     result: f64,
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Example: Add two numbers
#[derive(Debug, Deserialize)]
pub struct AddParams {
    pub a: i32,
    pub b: i32,
}

#[derive(Debug, Serialize)]
pub struct AddResult {
    pub sum: i32,
}

/// Example: Echo a message
#[derive(Debug, Deserialize)]
pub struct EchoParams {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct EchoResult {
    pub message: String,
}

/// Example: Get server status
#[derive(Debug, Deserialize)]
pub struct StatusParams {}

#[derive(Debug, Serialize)]
pub struct StatusResult {
    pub status: String,
    pub uptime_seconds: u64,
}

/// Example: Notification payload
#[derive(Debug, Serialize, Deserialize)]
pub struct EventNotification {
    pub event_type: String,
    pub data: serde_json::Value,
}



