# Quick Start Guide

## Running the Examples

### 1. Simple Server & Client

**Terminal 1 - Start the server:**
```bash
cargo run --example simple_server
```

**Terminal 2 - Run the client:**
```bash
cargo run --example simple_client
```

### 2. Bidirectional Communication

```bash
cargo run --example bidirectional
```

## Available Examples

JROW includes 23 comprehensive examples demonstrating all features:

### Basic Examples

- **`simple_server.rs` / `simple_client.rs`** - Basic RPC method handlers
- **`bidirectional.rs`** - Server-to-client notifications

### Publish/Subscribe

- **`pubsub.rs`** - Topic-based pub/sub with multiple clients
- **`pubsub_batch.rs`** - Batch subscribe/unsubscribe operations
- **`subscription_filters.rs`** - NATS-style pattern subscriptions with wildcard matching
- **`in_memory_pattern_matching.rs`** - Comprehensive NATS pattern matching demonstration

### Persistent Subscriptions

- **`persistent_server.rs` / `persistent_client.rs`** - Basic persistent subscriptions
- **`persistent_pubsub.rs`** - All-in-one persistent subscriptions demonstration
- **`persistent_batch.rs`** - Batch operations for persistent subscriptions
- **`persistent_pattern_matching.rs`** - All-in-one NATS-style pattern matching
- **`persistent_pattern_server.rs` / `persistent_pattern_client.rs`** - Pattern matching demonstration

### Batch Processing

- **`batch.rs`** - Batch requests with parallel/sequential modes
- **`publish_batch.rs`** - Server-side batch publishing to multiple topics

### Advanced Features

- **`middleware_example.rs`** - Request/response middleware with logging and metrics
- **`reconnection_client.rs` / `reconnection_server.rs`** - Automatic reconnection with configurable strategies
- **`observability_server.rs` / `observability_client.rs`** - OpenTelemetry distributed tracing
- **`observability_full.rs`** - Complete observability demo with all features

### Web UI

- **`server_with_ui.rs`** - Full-featured server with embedded web UI (all-in-one demo)

## Building the Project

```bash
# Check all crates compile
cargo check --all

# Run all tests
cargo test --all

# Build in release mode
cargo build --all --release
```

## Project Structure

```
jrow/
├── jrow-core/          # Core JSON-RPC types and codec
├── jrow-server/        # Server implementation
├── jrow-client/        # Client implementation
├── jrow-macros/        # Procedural macros
└── examples/           # Example applications (23 examples)
    ├── simple_server.rs
    ├── simple_client.rs
    ├── bidirectional.rs
    ├── pubsub.rs
    ├── pubsub_batch.rs
    ├── subscription_filters.rs
    ├── in_memory_pattern_matching.rs
    ├── persistent_server.rs
    ├── persistent_client.rs
    ├── persistent_pubsub.rs
    ├── persistent_batch.rs
    ├── persistent_pattern_matching.rs
    ├── persistent_pattern_server.rs
    ├── persistent_pattern_client.rs
    ├── batch.rs
    ├── publish_batch.rs
    ├── middleware_example.rs
    ├── reconnection_client.rs
    ├── reconnection_server.rs
    ├── observability_server.rs
    ├── observability_client.rs
    ├── observability_full.rs
    └── server_with_ui.rs
```

## Creating a Server

```rust
use jrow_server::{from_typed_fn, JrowServer};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Params { value: i32 }

#[derive(Serialize)]
struct Result { doubled: i32 }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = from_typed_fn(|params: Params| async move {
        Ok(Result { doubled: params.value * 2 })
    });

    let server = JrowServer::builder()
        .bind_str("127.0.0.1:8080")?
        .handler("double", handler)
        .build()
        .await?;

    server.run().await?;
    Ok(())
}
```

## Creating a Client

```rust
use jrow_client::JrowClient;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Params { value: i32 }

#[derive(Deserialize)]
struct Result { doubled: i32 }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = JrowClient::connect("ws://127.0.0.1:8080").await?;
    
    let result: Result = client
        .request("double", Params { value: 21 })
        .await?;
    
    println!("Result: {}", result.doubled); // 42
    Ok(())
}
```

## Features

- ✅ JSON-RPC 2.0 compliant
- ✅ WebSocket transport
- ✅ Type-safe handlers
- ✅ Async/await support
- ✅ Bidirectional notifications
- ✅ Request/response tracking
- ✅ Comprehensive error handling

## Next Steps

- Read the full [README.md](README.md)
- Explore the [examples/](examples/) directory
- Check the API documentation: `cargo doc --open`

