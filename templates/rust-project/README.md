# My JROW Application

A JSON-RPC 2.0 application built with [JROW](https://github.com/yourusername/jrow).

## Features

- ✅ JSON-RPC 2.0 over WebSocket
- ✅ Type-safe request/response handling
- ✅ Async/await with Tokio
- ✅ Batch requests support
- ✅ Publish/Subscribe
- ✅ Automatic reconnection
- ✅ Middleware system
- ✅ OpenTelemetry observability (optional)

## Quick Start

### Prerequisites

- Rust 1.75 or later
- Cargo

### Build

```bash
cargo build --release
```

### Run Server

```bash
# Using cargo
cargo run --bin server

# Or run the binary directly
./target/release/server

# With custom bind address
BIND_ADDRESS=0.0.0.0:9000 cargo run --bin server
```

### Run Client

In another terminal:

```bash
# Using cargo
cargo run --bin client

# Or run the binary directly
./target/release/client

# Connect to custom server
SERVER_URL=ws://localhost:9000 cargo run --bin client
```

## Project Structure

```
.
├── Cargo.toml           # Project dependencies and metadata
├── src/
│   ├── lib.rs          # Library root
│   ├── types.rs        # Request/response types
│   ├── handlers.rs     # RPC method handlers
│   └── bin/
│       ├── server.rs   # Server binary
│       └── client.rs   # Client binary
├── .env.example        # Environment variables template
├── .gitignore          # Git ignore patterns
└── README.md           # This file
```

## Available RPC Methods

### `add`

Add two numbers.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "add",
  "params": {"a": 5, "b": 3},
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {"sum": 8},
  "id": 1
}
```

### `echo`

Echo a message back.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "echo",
  "params": {"message": "Hello"},
  "id": 2
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {"message": "Hello"},
  "id": 2
}
```

### `status`

Get server status.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "status",
  "params": {},
  "id": 3
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "running",
    "uptime_seconds": 123
  },
  "id": 3
}
```

## Development

### Adding New Methods

1. Define types in `src/types.rs`:
   ```rust
   #[derive(Debug, Deserialize)]
   pub struct MyMethodParams {
       pub param1: String,
   }
   
   #[derive(Debug, Serialize)]
   pub struct MyMethodResult {
       pub result: String,
   }
   ```

2. Implement handler in `src/handlers.rs`:
   ```rust
   pub async fn my_method_handler(params: MyMethodParams) -> Result<MyMethodResult> {
       Ok(MyMethodResult {
           result: format!("Processed: {}", params.param1),
       })
   }
   ```

3. Register in `src/bin/server.rs`:
   ```rust
   .handler("my_method", from_typed_fn(my_method_handler))
   ```

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Linting

```bash
# Check code
cargo clippy

# Format code
cargo fmt
```

## Configuration

### Environment Variables

Copy `.env.example` to `.env` and customize:

```bash
cp .env.example .env
```

Available variables:
- `BIND_ADDRESS` - Server bind address (default: `127.0.0.1:8080`)
- `SERVER_URL` - Client connection URL (default: `ws://127.0.0.1:8080`)
- `RUST_LOG` - Log level (default: `info`)

### Features

Enable optional features by uncommenting in `Cargo.toml`:

- **Observability**: OpenTelemetry tracing, metrics, and logs
- **Configuration**: External config file support
- **Advanced logging**: Structured logging with tracing

## Deployment

### Docker

See [JROW deployment templates](https://github.com/yourusername/jrow/tree/main/templates/deploy) for:
- Dockerfile
- docker-compose.yml
- Kubernetes manifests

### Production Considerations

1. **Observability**: Enable OpenTelemetry for monitoring
2. **Security**: Use WSS (secure WebSocket) in production
3. **Performance**: Adjust batch size limits and connection timeouts
4. **Reliability**: Enable client reconnection strategies
5. **Scaling**: Run multiple server instances behind a load balancer

## Examples

See the [JROW examples](https://github.com/yourusername/jrow/tree/main/examples) for:
- Pub/sub patterns
- Batch requests
- Middleware
- Reconnection
- Observability

## Documentation

- [JROW Documentation](https://github.com/yourusername/jrow)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [Tokio Documentation](https://tokio.rs)

## License

MIT



