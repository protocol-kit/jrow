//! My JROW Application
//!
//! This is a template for building JSON-RPC applications with JROW.
//! It demonstrates the recommended project structure with separate
//! modules for types and handlers.
//!
//! # Project Structure
//!
//! - **types.rs**: Request/response types for RPC methods
//! - **handlers.rs**: Implementation of RPC method handlers
//! - **bin/server.rs**: Server entry point
//! - **bin/client.rs**: Example client application
//!
//! # Customization
//!
//! To add new RPC methods:
//! 1. Define request/response types in `types.rs`
//! 2. Implement handler function in `handlers.rs`
//! 3. Register handler in `bin/server.rs`
//!
//! # Examples
//!
//! See `bin/client.rs` for example usage of the defined methods.

pub mod handlers;
pub mod types;

// Re-export everything for convenience
pub use handlers::*;
pub use types::*;



