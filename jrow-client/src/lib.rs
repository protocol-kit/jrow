//! JSON-RPC 2.0 client implementation over WebSocket
//!
//! This crate provides a full-featured JSON-RPC 2.0 client that communicates
//! over WebSocket connections. It includes advanced features like automatic
//! reconnection, persistent subscriptions, batch requests, and observability.
//!
//! # Core Features
//!
//! - **WebSocket Transport**: Async WebSocket communication
//! - **Request-Response**: Send requests and await responses with type safety
//! - **Pub/Sub**: Subscribe to topics and receive notifications
//! - **Batch Requests**: Send multiple requests efficiently in one message
//! - **Auto-Reconnection**: Configurable reconnection with exponential backoff
//! - **Persistent Subscriptions**: Durable subscriptions with automatic resume
//! - **Observability**: OpenTelemetry integration for traces and metrics
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use jrow_client::JrowClient;
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Simple connection
//!     let client = JrowClient::connect("ws://localhost:8080").await?;
//!     
//!     // Make a request
//!     let result: serde_json::Value = client.request("ping", None::<serde_json::Value>).await?;
//!     println!("Result: {}", result);
//!     
//!     // Subscribe to notifications
//!     client.on_notification("events", |data| async move {
//!         println!("Event: {:?}", data);
//!     }).await;
//!     
//!     client.subscribe("events.*", |value| async move {
//!         println!("Received: {:?}", value);
//!     }).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! # With Reconnection
//!
//! ```rust,no_run
//! use jrow_client::{ClientBuilder, ExponentialBackoff};
//! use std::time::Duration;
//!
//! # async fn example() -> jrow_core::Result<()> {
//! let client = ClientBuilder::new("ws://localhost:8080")
//!     .with_reconnect(Box::new(
//!         ExponentialBackoff::new(
//!             Duration::from_millis(100),
//!             Duration::from_secs(30)
//!         )
//!         .with_max_attempts(10)
//!         .with_jitter()
//!     ))
//!     .connect()
//!     .await?;
//! # Ok(())
//! # }
//! ```

mod batch;
mod client;
mod client_builder;
mod connection_state;
mod metrics;
mod notification;
mod reconnect;
mod request;

pub use batch::{BatchRequest, BatchResponse};
pub use client::JrowClient;
pub use client_builder::ClientBuilder;
pub use connection_state::{ConnectionManager, ConnectionState};
pub use metrics::ClientMetrics;
pub use notification::NotificationHandler;
pub use reconnect::{ExponentialBackoff, FixedDelay, NoReconnect, ReconnectionStrategy};
