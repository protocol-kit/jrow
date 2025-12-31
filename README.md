# jrow - JSON-RPC over WebSocket Toolkit

[![Crates.io](https://img.shields.io/crates/v/jrow.svg)](https://crates.io/crates/jrow)
[![Documentation](https://docs.rs/jrow/badge.svg)](https://docs.rs/jrow)
[![License: MIT-0](https://img.shields.io/badge/License-MIT--0-blue.svg)](https://opensource.org/licenses/MIT-0)
[![License: CC0-1.0](https://img.shields.io/badge/License-CC0--1.0-blue.svg)](https://creativecommons.org/publicdomain/zero/1.0/)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/github/actions/workflow/status/protocol-kit/jrow/ci.yml?branch=main)](https://github.com/protocol-kit/jrow/actions)

IMPORTANT: Project in research and design phase. Drafts only.

A comprehensive JSON-RPC 2.0 implementation in Rust with both client and server support over WebSocket transport.

## Use Cases

JROW is ideal for:
- 🌐 **Web applications** with real-time features (chat, dashboards, notifications)
- 🤖 **IoT device control** and monitoring with bidirectional communication
- 🏢 **Microservices** internal RPC communication
- 💹 **Financial trading** platforms with ultra-low latency
- 💬 **Live collaboration** tools (editors, whiteboards)
- 📊 **Real-time dashboards** with bidirectional updates

**See [Use Cases and Technology Comparison](./docs/use-cases.md)** for detailed comparisons with NATS and Kafka, including decision matrices and hybrid architecture patterns.

## Features

- **Full JSON-RPC 2.0 compliance** - Supports requests, responses, notifications, and batches
- **WebSocket transport** - Built on tokio-tungstenite for high-performance async I/O
- **Type-safe handlers** - Automatic serialization/deserialization of parameters and results
- **Bidirectional communication** - Both client and server can send notifications
- **Publish/Subscribe** - Topic-based pub/sub with broadcast support and glob pattern filters
- **Persistent Subscriptions** - Exactly-once delivery with automatic recovery and state management
- **Batch Requests** - Send multiple requests in a single message with configurable processing
- **Automatic Reconnection** - Configurable strategies with automatic resubscription
- **Middleware System** - Pre/post request hooks with sync and async support
- **Configurable Limits** - Batch size limits and other safety features
- **OpenTelemetry** - Distributed tracing, metrics, and structured logging
- **Async/await** - First-class async support using Tokio
- **Modular architecture** - Separated into core, server, client, and macro crates

## Architecture

The toolkit is organized into four crates:

- **jrow-core** - Core JSON-RPC 2.0 types and codec
- **jrow-server** - Server implementation with routing and connection management
- **jrow-client** - Client implementation with request tracking and notification handling
- **jrow-macros** - Procedural macros for handler generation (planned)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
jrow-server = "0.1"
jrow-client = "0.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## Quick Commands

JROW includes a comprehensive Makefile for common tasks:

```bash
# Development
make build              # Build all crates
make test               # Run all tests
make check              # Quick check without building
make fmt                # Format code
make clippy             # Run linter

# Examples
make run-simple         # Run simple_server example
make run-pubsub         # Run pubsub example
make run-batch          # Run batch example
make run-server-ui      # Run server with embedded web UI (all-in-one!)

# Web UI
make run-web-ui         # Start web UI client on http://localhost:8000

# Documentation
make doc                # Generate Rust docs
make asyncapi-html      # Generate AsyncAPI docs

# Docker
make docker-build       # Build Docker image
make docker-compose-up  # Start with docker-compose

# Kubernetes
make k8s-apply          # Deploy to Kubernetes
make k8s-status         # Check deployment status

# All-in-one
make all                # Build, test, and lint
make pre-commit         # Run pre-commit checks
make help               # Show all available targets
```

## Deployment Templates

JROW includes **Tera-based deployment templates** for projects built with JROW:

- **Docker**: Customizable Dockerfile and docker-compose.yml
- **Kubernetes**: Deployment manifests with configurable resources
- **Template Generator**: CLI tool to render templates with your config

### Generate Deployment Files

```bash
# 1. Copy template configuration
cp templates/jrow-template.toml jrow-template.toml

# 2. Edit jrow-template.toml with your project details

# 3. Generate deployment files
make template-generate

# 4. Deploy
make deploy-docker    # or make deploy-k8s
```

The template generator creates customized deployment files for **your application** that uses JROW, not for JROW itself.

See [`templates/README.md`](templates/README.md) for detailed documentation.

## Quick Start

### Server

Create a simple JSON-RPC server:

```rust
use jrow_server::{from_typed_fn, JrowServer};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Serialize)]
struct AddResult {
    sum: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a handler
    let add_handler = from_typed_fn(|params: AddParams| async move {
        Ok(AddResult {
            sum: params.a + params.b,
        })
    });

    // Build and run the server
    let server = JrowServer::builder()
        .bind_str("127.0.0.1:8080")?
        .handler("add", add_handler)
        .build()
        .await?;

    server.run().await?;
    Ok(())
}
```

### Client

Connect to the server and make requests:

```rust
use jrow_client::JrowClient;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Deserialize)]
struct AddResult {
    sum: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = JrowClient::connect("ws://127.0.0.1:8080").await?;

    // Make a request
    let result: AddResult = client
        .request("add", AddParams { a: 5, b: 3 })
        .await?;

    println!("Result: {}", result.sum); // Prints: Result: 8

    // Send a notification (no response expected)
    client.notify("log", serde_json::json!({"message": "hello"})).await?;

    Ok(())
}
```

## Web UI Client

JROW includes a **beautiful web-based client** for testing and interacting with JROW servers - zero build required!

### Features

- 🌐 **Zero dependencies** - Pure HTML/CSS/JavaScript
- 🔌 **Connection management** - Connect to any JROW server
- 📤 **Request/Response** - Test JSON-RPC methods
- 📡 **Pub/Sub** - Subscribe to topics with pattern matching
- 📦 **Batch requests** - Send multiple requests at once
- 🖥️ **Real-time console** - View all WebSocket messages
- ⌨️ **Keyboard shortcuts** - Ctrl+Enter to send
- 🌙 **Dark theme** - Easy on the eyes

### Quick Start (All-in-One)

**Easiest option** - runs server + UI with one command:

```bash
# Start server with embedded web UI
make run-server-ui

# Opens on http://127.0.0.1:8080
# Automatically configured and ready to test!
```

### Alternative: Separate Server and UI

```bash
# Terminal 1: Start JROW server
make run-simple

# Terminal 2: Start web UI
make run-web-ui

# Browser: Connect to ws://localhost:8080 and test!
```

**Or open directly:**
```bash
open web-ui/index.html
```

See [`web-ui/README.md`](web-ui/README.md) for detailed usage guide.

## Advanced Usage

### Publish/Subscribe (Pub/Sub)

The toolkit includes built-in support for topic-based publish/subscribe patterns. Clients can subscribe to topics and receive broadcast notifications when the server publishes to those topics.

**Server-side publishing:**

```rust
// The server can publish messages to all subscribers of a topic
let count = server.publish("stock.prices", serde_json::json!({
    "symbol": "AAPL",
    "price": 150.0
})).await?;

println!("Published to {} subscribers", count);
```

**Client-side subscribing:**

```rust
let client = JrowClient::connect("ws://127.0.0.1:8080").await?;

// Subscribe to a topic with a handler
client.subscribe("stock.prices", |data| async move {
    println!("Received stock update: {}", data);
}).await?;

// The client will now receive all messages published to "stock.prices"

// Unsubscribe when done
client.unsubscribe("stock.prices").await?;
```

**Built-in RPC methods:**

- `rpc.subscribe` - Subscribe to a topic (supports glob patterns)
- `rpc.unsubscribe` - Unsubscribe from a topic

These methods are automatically available on all servers and handle subscription management, cleanup on disconnect, and acknowledgments.

**NATS-Style Pattern Matching:**

Subscribe to multiple topics with a single subscription using NATS-style patterns:

```rust
// Subscribe to single-level events (events.user, events.admin, etc.)
client.subscribe("events.*", |msg| async move {
    if let Some(obj) = msg.as_object() {
        let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("?");
        let data = obj.get("data").unwrap_or(&msg);
        println!("Event from {}: {}", topic, data);
    }
}).await?;

// Subscribe to multi-level events (events.user, events.user.login, etc.)
client.subscribe("events.>", |msg| async move {
    if let Some(obj) = msg.as_object() {
        let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("?");
        let data = obj.get("data").unwrap_or(&msg);
        println!("Deep event from {}: {}", topic, data);
    }
}).await?;

// Multiple single wildcards
client.subscribe("logs.*.error", |msg| async move {
    if let Some(obj) = msg.as_object() {
        let topic = obj.get("topic").and_then(|v| v.as_str()).unwrap_or("?");
        let data = obj.get("data").unwrap_or(&msg);
        println!("Error log from {}: {}", topic, data);
    }
}).await?;
```

**Pattern Syntax:**
- `*` matches exactly ONE token (e.g., `events.*` matches `events.user` but not `events.user.login`)
- `>` matches ONE OR MORE tokens (e.g., `events.>` matches `events.user` and `events.user.login`)
- Tokens are separated by `.`
- Multiple `*` wildcards are allowed in a pattern
- Only one `>` wildcard is allowed, and it must be the last token

For pattern subscriptions, the notification data includes the actual topic that matched.

See [`examples/subscription_filters.rs`](examples/subscription_filters.rs) for a complete demonstration.

**Batch subscribe/unsubscribe:**

For better performance when managing multiple subscriptions, use batch methods:

```rust
// Subscribe to multiple topics at once
let topics = vec![
    ("news".to_string(), |data| async move {
        println!("News: {}", data);
    }),
    ("alerts".to_string(), |data| async move {
        println!("Alert: {}", data);
    }),
    ("updates".to_string(), |data| async move {
        println!("Update: {}", data);
    }),
];

client.subscribe_batch(topics).await?;

// Unsubscribe from multiple topics at once
client.unsubscribe_batch(vec![
    "news".to_string(),
    "alerts".to_string(),
    "updates".to_string(),
]).await?;
```

Benefits:

- Single network round-trip for multiple subscriptions
- Atomic operation (all succeed or all fail for subscribe_batch)
- Significantly faster than individual calls

**Batch publish (server-side):**

For publishing to multiple topics at once, use the `publish_batch` method:

```rust
// Publish to multiple topics in a single operation
let messages = vec![
    ("news".to_string(), serde_json::json!({"title": "Breaking news"})),
    ("alerts".to_string(), serde_json::json!({"level": "warning"})),
    ("updates".to_string(), serde_json::json!({"version": "2.0"})),
];

let results = server.publish_batch(messages).await?;

// Check how many subscribers received each message
for (topic, count) in results {
    println!("'{}': {} subscribers notified", topic, count);
}
```

Benefits:

- Locks connection registry once instead of N times
- ~2.6x faster than individual publish calls
- Returns subscriber count for each topic

**Example pub/sub application:**

See [`examples/pubsub.rs`](examples/pubsub.rs) for a complete working example with multiple clients subscribing to different topics, [`examples/pubsub_batch.rs`](examples/pubsub_batch.rs) for batch subscription demonstrations, and [`examples/publish_batch.rs`](examples/publish_batch.rs) for batch publishing demonstrations.

```bash
cargo run --example pubsub
```

### Automatic Reconnection

JROW clients can automatically reconnect when connections are lost, with configurable strategies and automatic resubscription to topics.

**Basic usage:**

```rust
use jrow_client::{ClientBuilder, ExponentialBackoff};
use std::time::Duration;

// Create client with default reconnection strategy
let client = ClientBuilder::new("ws://127.0.0.1:8080")
    .with_default_reconnect()
    .connect()
    .await?;

// Or configure a custom strategy
let strategy = ExponentialBackoff::new(
    Duration::from_millis(100),  // min delay
    Duration::from_secs(30),     // max delay
)
.with_max_attempts(10)
.with_jitter();

let client = ClientBuilder::new("ws://127.0.0.1:8080")
    .with_reconnect(Box::new(strategy))
    .connect()
    .await?;
```

**Built-in strategies:**

- `ExponentialBackoff` - Exponential backoff with optional jitter (recommended)
- `FixedDelay` - Fixed delay between attempts
- `NoReconnect` - Disable reconnection (default)

**Features:**

- Automatic resubscription to all topics after reconnection
- Configurable retry limits and delays
- Connection state tracking
- Jitter support to prevent thundering herd

**Monitor connection state:**

```rust
use jrow_client::ConnectionState;

if let Some(state) = client.connection_state().await {
    match state {
        ConnectionState::Connected => println!("Connected"),
        ConnectionState::Reconnecting { attempt } => {
            println!("Reconnecting (attempt {})", attempt)
        }
        ConnectionState::Failed => println!("Reconnection failed"),
        _ => {}
    }
}
```

See [docs/reconnection.md](./docs/reconnection.md) for detailed documentation and examples.

### Persistent Subscriptions

Persistent subscriptions provide reliable, exactly-once message delivery with automatic recovery and state management. Messages are stored in a durable database and automatically delivered even after disconnection.

**Server setup with persistent storage:**

```rust
use jrow_server::{JrowServer, RetentionPolicy};
use std::time::Duration;

let server = JrowServer::builder()
    .bind_str("127.0.0.1:9004")?
    .with_persistent_storage("./data/events.db")
    .register_topic(
        "events",
        RetentionPolicy {
            max_age: Some(Duration::from_secs(3600)),     // Keep 1 hour
            max_count: Some(1000),                         // Keep last 1000
            max_bytes: Some(10 * 1024 * 1024),            // Keep 10MB
        },
    )
    .subscription_timeout(Duration::from_secs(300))
    .retention_interval(Duration::from_secs(60))
    .build()
    .await?;

// Publish persistent messages
let seq_id = server
    .publish_persistent("events", serde_json::json!({
        "event": "user.created",
        "user_id": 123
    }))
    .await?;

println!("Published with sequence: {}", seq_id);
```

**Client subscription with automatic resume:**

```rust
let client = JrowClient::connect("ws://127.0.0.1:9004").await?;

// Subscribe with a unique subscription ID
// On reconnect, automatically resumes from last acknowledged position
let resumed_seq = client
    .subscribe_persistent("event-processor-1", "events", |msg| {
        let client_clone = client.clone();
        async move {
            if let Some(obj) = msg.as_object() {
                if let Some(seq_id) = obj.get("sequence_id").and_then(|v| v.as_u64()) {
                    // Process message
                    process_event(msg).await;
                    
                    // Acknowledge (non-blocking, spawns internally)
                    client_clone.ack_persistent("event-processor-1", seq_id);
                }
            }
        }
    })
    .await?;

println!("Resumed from sequence: {}", resumed_seq);
```

**Key features:**

- ✅ **Exactly-Once Delivery** - Each message processed exactly once
- ✅ **Automatic Resume** - Picks up from last acknowledged position
- ✅ **Persistent Storage** - Built on sled embedded database
- ✅ **Retention Policies** - Automatic cleanup based on age, count, or size
- ✅ **Manual Acknowledgment** - Messages redelivered until acknowledged
- ✅ **Exclusive Subscriptions** - One connection per subscription ID at a time

**Use cases:**

- Event processing pipelines
- Task queues with guaranteed delivery
- Audit log processing
- Order processing systems
- Data synchronization

**Example applications:**

Run the persistent pub/sub examples:

```bash
# Terminal 1: Start server (publishes events every 10 seconds)
cargo run --example persistent_server

# Terminal 2: Start client (processes events)
cargo run --example persistent_client

# Stop the client, wait, then restart - it automatically resumes!
```

See [`docs/persistent-subscriptions.md`](docs/persistent-subscriptions.md) for complete documentation and [`examples/persistent_pubsub.rs`](examples/persistent_pubsub.rs) for an all-in-one demonstration.

**Batch operations for persistent subscriptions:**

For high-throughput scenarios, batch operations reduce network overhead when managing multiple subscriptions or acknowledging multiple messages:

```rust
// Subscribe to multiple persistent subscriptions at once
let resumed_seqs = client
    .subscribe_persistent_batch(vec![
        ("order-processor".to_string(), "orders".to_string(), order_handler),
        ("payment-processor".to_string(), "payments".to_string(), payment_handler),
        ("notification-processor".to_string(), "notifications".to_string(), notif_handler),
    ])
    .await?;

// Each subscription resumes from its last acknowledged position
for (sub_id, seq) in resumed_seqs {
    println!("{} resumed from sequence {}", sub_id, seq);
}

// Batch acknowledge multiple messages (fire-and-forget)
client.ack_persistent_batch(vec![
    ("order-processor".to_string(), 101),
    ("order-processor".to_string(), 102),
    ("payment-processor".to_string(), 55),
]);

// Or await the acknowledgment results
let ack_results = client.ack_persistent_batch_await(vec![
    ("order-processor".to_string(), 103),
    ("payment-processor".to_string(), 56),
]).await?;

for (sub_id, seq_id, success) in ack_results {
    println!("Ack {}/{}: {}", sub_id, seq_id, success);
}

// Batch unsubscribe from multiple subscriptions
client.unsubscribe_persistent_batch(vec![
    "order-processor".to_string(),
    "payment-processor".to_string(),
    "notification-processor".to_string(),
]).await?;
```

**Benefits of batching:**

- **Performance** - Single network round-trip for multiple operations
- **Efficiency** - Reduced database write operations for acknowledgments
- **Throughput** - Handle high-volume message processing with lower overhead
- **Consistency** - Atomic operations where possible

**Example:**

See [`examples/persistent_batch.rs`](examples/persistent_batch.rs) for a complete demonstration:

```bash
cargo run --example persistent_batch
```

**NATS-style pattern matching:**

Persistent subscriptions support NATS-style pattern matching, allowing you to subscribe to multiple topics with a single subscription using wildcards:

```rust
// Subscribe to all order events with a single subscription
client.subscribe_persistent("all-orders", "orders.*", |msg| {
    async move {
        // Receives: orders.new, orders.shipped, orders.cancelled, etc.
        process_order(msg).await;
    }
}).await?;

// Subscribe to deeply nested events
client.subscribe_persistent("user-events", "events.user.>", |msg| {
    async move {
        // Receives: events.user, events.user.login, events.user.login.success, etc.
        process_user_event(msg).await;
    }
}).await?;

// Subscribe to specific patterns
client.subscribe_persistent("successes", "*.*.success", |msg| {
    async move {
        // Receives: auth.login.success, payment.charge.success, etc.
        track_success(msg).await;
    }
}).await?;
```

**Pattern syntax:**

- `.` (dot) - Token delimiter
- `*` - Matches exactly ONE token at that position
- `>` - Matches ONE OR MORE tokens (must be at end)

**Examples:**

| Pattern | Matches | Doesn't Match |
|---------|---------|---------------|
| `orders.new` | `orders.new` only | `orders.shipped` |
| `orders.*` | `orders.new`, `orders.shipped` | `orders`, `orders.new.fast` |
| `orders.>` | `orders.new`, `orders.new.shipped` | `orders`, `events.new` |
| `*.login` | `user.login`, `admin.login` | `user.logout` |

See [`docs/nats-pattern-matching.md`](docs/nats-pattern-matching.md) for complete pattern matching documentation.

**Examples:**

```bash
# All-in-one demonstration
cargo run --example persistent_pattern_matching

# Client-server demonstration (run in separate terminals)
# Terminal 1: Start server
cargo run --example persistent_pattern_server

# Terminal 2: Start client (try stopping and restarting to see resume!)
cargo run --example persistent_pattern_client
```

### Batch Requests

Send multiple requests in a single message to reduce network overhead and latency. The server can process batches in parallel or sequentially.

**Server configuration:**

```rust
use jrow_server::BatchMode;

let server = JrowServer::builder()
    .bind_str("127.0.0.1:8080")?
    .batch_mode(BatchMode::Parallel)  // or BatchMode::Sequential
    .max_batch_size(100)                // Limit batch size (optional, prevents DoS)
    .handler("add", add_handler)
    .handler("multiply", multiply_handler)
    .build()
    .await?;
```

**Batch Size Limits:**

To prevent resource exhaustion and DoS attacks, configure a maximum batch size:

```rust
let server = JrowServer::builder()
    .bind_str("127.0.0.1:8080")?
    .max_batch_size(50)  // Reject batches larger than 50 requests
    .build()
    .await?;
```

If a client sends a batch exceeding the limit, the server returns a JSON-RPC error with code `-32600` (Invalid Request).

**Client batch requests:**

```rust
use jrow_client::BatchRequest;

let mut batch = BatchRequest::new();

// Add requests to the batch
let id1 = batch.add_request("add", AddParams { a: 1, b: 2 });
let id2 = batch.add_request("add", AddParams { a: 5, b: 3 });
let id3 = batch.add_request("multiply", MulParams { a: 3, b: 4 });

// Add notifications (no response)
batch.add_notification("log", LogParams { msg: "Batch sent" });

// Send the batch
let responses = client.batch(batch).await?;

// Extract typed results
let sum1: i32 = responses.get(&id1)?;
let sum2: i32 = responses.get(&id2)?;
let product: i32 = responses.get(&id3)?;

println!("Results: {}, {}, {}", sum1, sum2, product);
```

**Benefits:**

- Reduces network round-trips (1 message instead of N)
- Lower latency for multiple requests
- Configurable processing (parallel for speed, sequential for order)
- Partial failure handling (some requests can fail while others succeed)

**Example:**

See [`examples/batch.rs`](examples/batch.rs) for a complete working example with performance comparisons.

```bash
cargo run --example batch
```

### Middleware System

Add middleware to intercept and process requests/responses for logging, authentication, rate limiting, metrics, and more.

**Built-in middleware:**

```rust
use jrow_server::{LoggingMiddleware, MetricsMiddleware};

let metrics = Arc::new(MetricsMiddleware::new());

let server = JrowServer::builder()
    .bind_str("127.0.0.1:8080")?
    .use_sync_middleware(LoggingMiddleware::new())
    .use_middleware(metrics.clone())
    .handler("add", add_handler)
    .build()
    .await?;

// Access metrics
println!("Total requests: {}", metrics.get_request_count());
```

**Custom middleware:**

```rust
use jrow_server::{SyncMiddleware, MiddlewareAction, MiddlewareContext};

struct AuthMiddleware;

impl SyncMiddleware for AuthMiddleware {
    fn pre_handle(&self, ctx: &mut MiddlewareContext) -> Result<MiddlewareAction> {
        // Check authentication
        if !is_authenticated(ctx.conn_id) {
            // Short-circuit and return error
            return Ok(MiddlewareAction::ShortCircuit(
                serde_json::json!({"error": "Unauthorized"})
            ));
        }
        Ok(MiddlewareAction::Continue)
    }

    fn post_handle(&self, _ctx: &mut MiddlewareContext, _result: &Result<Value>) -> Result<()> {
        Ok(())
    }
}

let server = JrowServer::builder()
    .bind_str("127.0.0.1:8080")?
    .use_sync_middleware(AuthMiddleware)
    .handler("protected_method", handler)
    .build()
    .await?;
```

**Features:**
- Sync and async middleware support
- Pre and post-request hooks
- Short-circuit capability (skip handler)
- Metadata passing between middleware
- Execution order control

See [`examples/middleware_example.rs`](examples/middleware_example.rs) for a complete example and [docs/middleware.md](docs/middleware.md) for detailed documentation.

### Multiple Handlers

Register multiple handlers with the server:

```rust
let server = JrowServer::builder()
    .bind_str("127.0.0.1:8080")?
    .handler("add", add_handler)
    .handler("subtract", subtract_handler)
    .handler("multiply", multiply_handler)
    .build()
    .await?;
```

### Notification Handling

Register handlers for incoming notifications on the client:

```rust
let client = JrowClient::connect("ws://127.0.0.1:8080").await?;

client.on_notification("status_update", |notif| async move {
    println!("Received notification: {:?}", notif);
}).await;
```

### Error Handling

The toolkit uses standard Result types and provides JSON-RPC error codes:

```rust
use jrow_core::{Error, Result};

let handler = from_typed_fn(|params: MyParams| async move {
    if params.value < 0 {
        return Err(Error::InvalidParams("Value must be positive".to_string()));
    }
    Ok(MyResult { value: params.value * 2 })
});
```

### Custom Router

Build a router separately and use it with the server:

```rust
use jrow_server::{RouterBuilder, from_typed_fn};

let router = RouterBuilder::new()
    .handler("method1", handler1)
    .handler("method2", handler2)
    .build();

let server = JrowServer::builder()
    .bind_str("127.0.0.1:8080")?
    .router(router)
    .build()
    .await?;
```

## JSON-RPC 2.0 Compliance

The toolkit strictly follows the [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification):

- **Request**: Contains `jsonrpc`, `method`, `params` (optional), and `id`
- **Notification**: Like a request but without `id` (no response expected)
- **Response**: Contains `jsonrpc`, `result` or `error`, and `id`
- **Error codes**: Standard codes from -32700 to -32603

### Standard Error Codes

- `-32700` Parse error - Invalid JSON
- `-32600` Invalid request - Not a valid JSON-RPC request
- `-32601` Method not found
- `-32602` Invalid params
- `-32603` Internal error

## API Documentation

JROW includes customizable [AsyncAPI 3.0](https://www.asyncapi.com/) specification templates for documenting your WebSocket JSON-RPC API.

### Features

- **Customizable Templates**: Define your RPC methods, topics, and server configurations
- **Complete Documentation**: All RPC methods, notifications, and pub/sub topics
- **Interactive Tools**: AsyncAPI Studio support
- **Code Generation**: Generate client SDKs in multiple languages
- **Industry Standard**: AsyncAPI 3.0 specification

### Quick Start

1. **Configure your API** in `jrow-template.toml`:

```toml
[asyncapi]
production_host = "api.example.com"
production_port = 443

[[asyncapi.methods]]
name = "getUserProfile"
example_params = '{"userId": "123"}'
example_result = '{"id": "123", "name": "Alice"}'

[[asyncapi.topics]]
name = "chat.messages"
example_params = '{"user": "alice", "message": "Hello"}'
```

2. **Generate the specification**:

```bash
make template-generate
```

3. **Use the AsyncAPI tools**:

```bash
# Validate specification
asyncapi validate templates/asyncapi.yaml

# Generate HTML documentation
asyncapi generate fromTemplate templates/asyncapi.yaml @asyncapi/html-template -o docs/

# Start AsyncAPI Studio
asyncapi start studio templates/asyncapi.yaml

# Generate client code
asyncapi generate fromTemplate templates/asyncapi.yaml @asyncapi/nodejs-template -o client/
```

See [`templates/README.md`](templates/README.md) for detailed AsyncAPI documentation.

## Examples

The `examples/` directory contains 23 comprehensive working examples demonstrating all JROW features:

### Basic Examples

- **`simple_server.rs`** - Basic server with multiple RPC method handlers
- **`simple_client.rs`** - Client making various requests and handling responses
- **`bidirectional.rs`** - Bidirectional communication with server-to-client notifications

### Publish/Subscribe Examples

- **`pubsub.rs`** - Topic-based pub/sub with multiple clients and broadcast
- **`pubsub_batch.rs`** - Batch subscribe/unsubscribe operations for multiple topics
- **`persistent_server.rs`** - Server publishing persistent messages every 10 seconds
- **`persistent_client.rs`** - Client with persistent subscription and auto-resume
- **`persistent_pubsub.rs`** - All-in-one persistent subscriptions demonstration
- **`persistent_batch.rs`** - Batch operations for persistent subscriptions (subscribe, ack, unsubscribe)
- **`persistent_pattern_matching.rs`** - All-in-one NATS-style pattern matching demonstration
- **`persistent_pattern_server.rs`** - Server publishing to various topics for pattern matching
- **`persistent_pattern_client.rs`** - Client using patterns to subscribe to multiple topics

### Batch Processing Examples

- **`batch.rs`** - Batch requests with parallel/sequential processing modes
- **`publish_batch.rs`** - Server-side batch publishing to multiple topics

### Advanced Features Examples

- **`middleware_example.rs`** - Request/response middleware with logging and metrics
- **`subscription_filters.rs`** - NATS-style pattern subscriptions with wildcard matching
- **`in_memory_pattern_matching.rs`** - Comprehensive NATS pattern matching demonstration
- **`reconnection_client.rs` / `reconnection_server.rs`** - Automatic reconnection with configurable strategies
- **`observability_server.rs` / `observability_client.rs`** - OpenTelemetry distributed tracing
- **`observability_full.rs`** - Complete observability demo with all features

### Web UI Example

- **`server_with_ui.rs`** - Full-featured server with embedded web UI (all-in-one demo)

### Running Examples

**Client-Server Examples** (require two terminals):

```bash
# Terminal 1: Start the server
cargo run --example simple_server

# Terminal 2: Run the client
cargo run --example simple_client
```

**Self-Contained Examples** (run in one terminal):

```bash
# Pub/sub with multiple clients
cargo run --example pubsub

# Batch requests with performance comparison
cargo run --example batch

# Batch subscribe/unsubscribe operations
cargo run --example pubsub_batch

# Batch publish to multiple topics
cargo run --example publish_batch

# Middleware system
cargo run --example middleware_example

# Subscription filters with glob patterns
cargo run --example subscription_filters

# Persistent subscriptions with exactly-once delivery
cargo run --example persistent_server      # Terminal 1
cargo run --example persistent_client      # Terminal 2
cargo run --example persistent_pubsub      # All-in-one

# OpenTelemetry observability
cargo run --example observability_server   # Terminal 1
cargo run --example observability_client   # Terminal 2
cargo run --example observability_full     # All-in-one
```

**Reconnection Example** (requires two terminals):

```bash
# Terminal 1: Start the server
cargo run --example reconnection_server

# Terminal 2: Run the client
cargo run --example reconnection_client

# Try stopping and restarting the server to see automatic reconnection
```

**Using Make targets:**

```bash
make run-simple         # Run simple_server
make run-pubsub         # Run pubsub example
make run-batch          # Run batch example
```

## Observability

JROW includes comprehensive OpenTelemetry support for production monitoring.

### Quick Start

```bash
# Start observability stack (Jaeger, Prometheus, Grafana)
docker-compose -f templates/deploy/observability/docker-compose.observability.yml up -d

# Enable in server
let server = ServerBuilder::new()
    .bind(addr)
    .with_observability(ObservabilityConfig::new("my-service"))
    .build()
    .await?;

# Enable in client
let client = ClientBuilder::new(url)
    .with_observability(ObservabilityConfig::new("my-client"))
    .connect()
    .await?;
```

### Features

- **Distributed Tracing**: Track requests across client-server-middleware
- **Metrics**: Connections, requests, batch sizes, pub/sub activity
- **Structured Logs**: JSON logs with trace context
- **Automatic Instrumentation**: Spans for all operations
- **OTLP Export**: Compatible with Jaeger, Tempo, Datadog, etc.

### View Telemetry

- **Jaeger UI**: http://localhost:16686 (traces)
- **Prometheus**: http://localhost:9090 (metrics)
- **Grafana**: http://localhost:3000 (dashboards)

See [docs/observability.md](docs/observability.md) for complete documentation.

## Testing

Run the test suite:

```bash
cargo test --all
```

Run tests with output:

```bash
cargo test --all -- --nocapture
```

## Architecture Details

### Core Layer (jrow-core)

Provides the fundamental JSON-RPC 2.0 types:

- `Id` - Request ID (string, number, or null)
- `JsonRpcRequest` - Request message
- `JsonRpcResponse` - Response message
- `JsonRpcNotification` - Notification message
- `JsonRpcError` - Error details
- `codec` - Serialization/deserialization functions

### Server Layer (jrow-server)

- **Router** - Maps method names to handlers
- **Handler** - Trait for implementing method handlers
- **Connection** - Manages WebSocket connections
- **SubscriptionManager** - Manages topic subscriptions and broadcasting
- **ServerBuilder** - Fluent API for server construction

### Client Layer (jrow-client)

- **JrowClient** - Main client interface
- **RequestManager** - Tracks pending requests and matches responses
- **NotificationHandler** - Handles incoming notifications

## Performance Considerations

- Handlers run concurrently using Tokio tasks
- WebSocket connections are non-blocking
- Request/response matching uses efficient hash maps
- Serialization is done with serde_json (fast and widely used)

## Limitations

- No built-in authentication (can be added via middleware)

## License

This project is MIT-0 licensed for code and CC0-1.0 licensed for non-code content - See LICENSE file for details

## Specification

For the complete formal specification of JSON-RPC over WebSocket protocol, see:

**[JSON-RPC over WebSocket Specification](./docs/SPECIFICATION.md)**

This specification defines:
- WebSocket transport requirements
- Message format and encoding
- Connection lifecycle
- Request/response patterns
- Batch processing
- Error handling
- Pub/Sub extension
- Persistent subscriptions extension
- Security considerations

**[Specification Compliance Report](./docs/SPECIFICATION-COMPLIANCE.md)** - ✅ **JROW is 100% compliant** with the specification, with full verification and testing coverage

## Resources

- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [WebSocket RFC](https://tools.ietf.org/html/rfc6455)
- [JROW Protocol Specification](./docs/SPECIFICATION.md)
