//! Core JSON-RPC 2.0 types and codec for jrow
//!
//! This crate provides the foundational types and utilities for implementing
//! JSON-RPC 2.0 communication. It includes:
//!
//! - **Types**: Core JSON-RPC 2.0 data structures (requests, responses, notifications)
//! - **Codec**: Serialization and deserialization utilities for JSON-RPC messages
//! - **Error handling**: Comprehensive error types for JSON-RPC operations
//! - **Observability**: OpenTelemetry integration for distributed tracing, metrics, and logs
//!
//! # Overview
//!
//! The JSON-RPC 2.0 specification defines a stateless, light-weight remote procedure
//! call (RPC) protocol. This crate provides a complete, spec-compliant implementation
//! of the protocol's data structures and encoding/decoding logic.
//!
//! # Architecture
//!
//! The crate is designed to be transport-agnostic - it handles message serialization
//! and deserialization but doesn't dictate how messages are transported. The `jrow-server`
//! and `jrow-client` crates build on top of this foundation to provide WebSocket-based
//! transport implementations.
//!
//! # Example
//!
//! ```rust
//! use jrow_core::{JsonRpcRequest, JsonRpcResponse, Id, codec};
//!
//! // Create a request
//! let request = JsonRpcRequest::new("add", Some(serde_json::json!({"a": 5, "b": 3})), Id::Number(1));
//!
//! // Encode it to JSON
//! let json = codec::encode_request(&request).unwrap();
//!
//! // Decode it back
//! let decoded = codec::decode_request(&json).unwrap();
//! assert_eq!(decoded.method, "add");
//! ```

pub mod codec;
pub mod error;
pub mod observability;
pub mod types;

// Re-export the most commonly used types for convenience
// This allows users to use `jrow_core::Error` instead of `jrow_core::error::Error`
pub use error::{Error, JsonRpcErrorData, Result};
pub use observability::{init_observability, shutdown_observability, ObservabilityConfig};
pub use types::{
    Id, JsonRpcError, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
