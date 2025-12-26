//! JROW - JSON-RPC 2.0 Over WebSocket
//!
//! This is the main convenience crate that re-exports all JROW sub-crates.
//! Use this crate if you want a single dependency that provides both client
//! and server functionality.
//!
//! # Architecture
//!
//! JROW is organized into modular crates:
//!
//! - **jrow-core**: Core types, codec, error handling, observability
//! - **jrow-server**: WebSocket JSON-RPC server with pub/sub
//! - **jrow-client**: WebSocket JSON-RPC client with reconnection
//! - **jrow-macros**: Procedural macros for handler generation
//!
//! # Quick Start - Server
//!
//! ```rust,no_run
//! use jrow::JrowServer;
//! use jrow::server::from_typed_fn;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct AddParams { a: i32, b: i32 }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let addr: std::net::SocketAddr = "127.0.0.1:8080".parse()?;
//!     let server = JrowServer::builder()
//!         .bind(addr)
//!         .handler("add", from_typed_fn(|p: AddParams| async move {
//!             Ok(p.a + p.b)
//!         }))
//!         .build()
//!         .await?;
//!     
//!     server.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Quick Start - Client
//!
//! ```rust,no_run
//! use jrow::JrowClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = JrowClient::connect("ws://localhost:8080").await?;
//!     
//!     let result: serde_json::Value = client.request("add", serde_json::json!({"a": 5, "b": 3})).await?;
//!     println!("Result: {}", result);
//!     
//!     Ok(())
//! }
//! ```

// Re-export all public APIs from sub-crates
// This allows users to access everything through `jrow::` prefix
pub use jrow_client as client;
pub use jrow_core as core;
pub use jrow_macros as macros;
pub use jrow_server as server;

// Convenience re-exports of the most commonly used types
// This avoids needing to write `jrow::server::JrowServer`
pub use jrow_client::JrowClient;
pub use jrow_server::JrowServer;


