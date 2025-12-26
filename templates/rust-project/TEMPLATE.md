# JROW Rust Project Template

This is a ready-to-use Rust project template for building JSON-RPC applications with JROW.

## What's Included

### Project Structure

```
rust-project/
├── Cargo.toml              # Project configuration with JROW dependencies
├── Makefile                # Common development tasks
├── README.md               # Project documentation (customizable)
├── rust-toolchain.toml     # Rust version specification
├── .gitignore              # Git ignore patterns
├── .env.example            # Environment variables template
└── src/
    ├── lib.rs             # Library root
    ├── types.rs           # Request/response type definitions
    ├── handlers.rs        # RPC method handler implementations
    └── bin/
        ├── server.rs      # Server binary
        └── client.rs      # Client example binary
```

### Features

- ✅ **Complete server and client** implementation
- ✅ **Type-safe RPC handlers** with serde serialization
- ✅ **Example methods**: `add`, `echo`, `status`
- ✅ **Async/await** with Tokio runtime
- ✅ **Environment configuration** support
- ✅ **Makefile** for common tasks
- ✅ **Production-ready** with release optimizations
- ✅ **Ready for extension** with:
  - Batch requests
  - Pub/sub
  - Middleware
  - Observability
  - Reconnection

## Usage

### 1. Copy the Template

```bash
# From the JROW repository root
cp -r templates/rust-project my-jrow-app
cd my-jrow-app
```

### 2. Customize

Edit `Cargo.toml` to change:
- Package name
- Version
- Authors
- Description
- Repository URL

```toml
[package]
name = "my-awesome-app"  # Change this
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
description = "My awesome JROW application"
repository = "https://github.com/yourusername/my-awesome-app"
```

### 3. Set Up Environment

```bash
# Copy environment template
cp .env.example .env

# Edit .env with your configuration
vim .env
```

### 4. Build and Run

```bash
# Build the project
make build

# Run server (terminal 1)
make run-server

# Run client (terminal 2)
make run-client
```

## Extending the Template

### Adding New RPC Methods

1. **Define types** in `src/types.rs`:

```rust
#[derive(Debug, Deserialize)]
pub struct CalculateParams {
    pub operation: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct CalculateResult {
    pub result: f64,
}
```

2. **Implement handler** in `src/handlers.rs`:

```rust
pub async fn calculate_handler(params: CalculateParams) -> Result<CalculateResult> {
    let result = match params.operation.as_str() {
        "sum" => params.values.iter().sum(),
        "avg" => params.values.iter().sum::<f64>() / params.values.len() as f64,
        _ => return Err(jrow_core::Error::InvalidParams("Unknown operation".into())),
    };
    Ok(CalculateResult { result })
}
```

3. **Register handler** in `src/bin/server.rs`:

```rust
.handler("calculate", from_typed_fn(calculate_handler))
```

### Enabling Optional Features

#### Observability (OpenTelemetry)

Uncomment in `src/bin/server.rs`:
```rust
.with_default_observability()
```

Uncomment in `src/bin/client.rs`:
```rust
.with_default_observability()
```

#### Batch Processing

Uncomment in `src/bin/server.rs`:
```rust
.batch_mode(BatchMode::Parallel)
.max_batch_size(100)
```

#### Middleware

Uncomment in `src/bin/server.rs`:
```rust
.use_middleware(Arc::new(LoggingMiddleware::new()))
```

#### Reconnection

Uncomment in `src/bin/client.rs`:
```rust
.with_default_reconnect()
```

### Adding Pub/Sub

1. **Subscribe on client**:

```rust
client.subscribe("events", |notification| {
    Box::pin(async move {
        println!("Received: {:?}", notification);
    })
}).await?;
```

2. **Publish from server**:

```rust
server.publish("events", serde_json::json!({
    "type": "user_registered",
    "user_id": 123
})).await?;
```

## Development Workflow

```bash
# Format code
make fmt

# Run linter
make clippy

# Run tests
make test

# Auto-reload on changes (requires cargo-watch)
make dev

# Build for production
make release
```

## Deployment

### Using JROW Templates

Generate deployment configurations:

```bash
# From JROW repository
cp templates/jrow-template.toml jrow-template.toml
# Edit jrow-template.toml with your app details
make template-generate
```

This generates:
- Docker files
- Kubernetes manifests
- Deployment scripts
- AsyncAPI documentation

### Docker

```bash
make docker-build
make docker-run
```

### Kubernetes

See generated manifests in `deploy/kubernetes/` after running `make template-generate`.

## Best Practices

1. **Error Handling**: Use proper error types from `jrow_core::Error`
2. **Async Operations**: Always use `.await` for async operations
3. **Type Safety**: Define strong types for all RPC methods
4. **Testing**: Add unit tests for handlers
5. **Documentation**: Update README.md for your specific application
6. **Security**: Use WSS (secure WebSocket) in production
7. **Monitoring**: Enable observability for production deployments

## Examples

This template includes 3 example RPC methods:

### `add` - Simple calculation
```bash
# Request
{"jsonrpc":"2.0","method":"add","params":{"a":5,"b":3},"id":1}
# Response
{"jsonrpc":"2.0","result":{"sum":8},"id":1}
```

### `echo` - Message echo
```bash
# Request
{"jsonrpc":"2.0","method":"echo","params":{"message":"Hello"},"id":2}
# Response
{"jsonrpc":"2.0","result":{"message":"Hello"},"id":2}
```

### `status` - Server information
```bash
# Request
{"jsonrpc":"2.0","method":"status","params":{},"id":3}
# Response
{"jsonrpc":"2.0","result":{"status":"running","uptime_seconds":123},"id":3}
```

## Troubleshooting

### Server won't start

- Check if port 8080 is available
- Try a different port: `BIND_ADDRESS=127.0.0.1:9000 make run-server`

### Client can't connect

- Ensure server is running
- Check SERVER_URL in .env
- Verify no firewall blocking the connection

### Build errors

- Ensure Rust 1.75 or later: `rustc --version`
- Update dependencies: `cargo update`
- Clean build: `make clean && make build`

## Resources

- [JROW Documentation](https://github.com/yourusername/jrow)
- [JROW Examples](https://github.com/yourusername/jrow/tree/main/examples)
- [JSON-RPC 2.0 Spec](https://www.jsonrpc.org/specification)
- [Tokio Guide](https://tokio.rs/tokio/tutorial)

## License

This template is part of JROW and follows the same license.



