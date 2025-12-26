# Batch Publish Implementation

## Overview

This document describes the implementation of the `publish_batch` method for the JROW server, which allows publishing messages to multiple topics in a single operation.

## Implementation Date

December 23, 2025

## Motivation

When publishing to multiple topics, making individual `publish()` calls creates unnecessary overhead:

- **Multiple lock acquisitions**: N topics = N connection registry locks
- **Higher latency**: Sequential publishes add up
- **Less efficient**: Repeated lookups and iterations

The batch publish feature addresses these issues by locking the connection registry once and processing all publishes in a single operation.

## Features

### Batch Publish (`publish_batch`)

Publish messages to multiple topics at once with a single operation.

**Signature:**
```rust
pub async fn publish_batch(&self, messages: Vec<(String, serde_json::Value)>) -> Result<Vec<(String, usize)>>
```

**Behavior:**
- Locks the connection registry once for all publishes
- Iterates through all topics and their subscribers
- Sends notifications to all subscribers of each topic
- Returns a vector of (topic, subscriber_count) pairs in the same order as input

**Example:**
```rust
let messages = vec![
    ("news".to_string(), serde_json::json!({"title": "Breaking news"})),
    ("alerts".to_string(), serde_json::json!({"level": "warning"})),
    ("updates".to_string(), serde_json::json!({"version": "2.0"})),
];

let results = server.publish_batch(messages).await?;

for (topic, count) in results {
    println!("'{}': {} subscribers notified", topic, count);
}
```

## Performance Benefits

```mermaid
graph TB
    subgraph Individual["Individual Publishes (60.9µs)"]
        I1[Lock Registry] --> P1[Publish Topic 1]
        P1 --> U1[Unlock]
        U1 --> I2[Lock Registry]
        I2 --> P2[Publish Topic 2]
        P2 --> U2[Unlock]
        U2 --> I3[... 8 more ...]
    end
    
    subgraph Batch["Batch Publish (23.2µs)"]
        B1[Lock Registry Once] --> BP1[Publish Topic 1]
        BP1 --> BP2[Publish Topic 2]
        BP2 --> BP3[Publish Topics 3-10]
        BP3 --> BU[Unlock Once]
    end
    
    style Individual fill:#FFCCBC
    style Batch fill:#C8E6C9
```

Based on the example demonstration:

- **~2.6x faster** than individual publish calls (10 topics: 60.9µs individual vs 23.2µs batch)
- **Single lock acquisition** instead of N
- **Reduced overhead** from fewer async operations

## Implementation Details

### Files Modified

1. **`jrow-server/src/lib.rs`** (60 lines added, 1 line changed)
   - Added `publish_batch()` method
   - Changed `run()` from `self` to `&self` to allow shared access
   - Locks connection registry once for all publishes

2. **`README.md`** (28 lines added)
   - Added documentation for batch publish
   - Added usage examples
   - Added performance benefits section
   - Updated examples list

3. **`examples/publish_batch.rs`** (new file, 158 lines)
   - Comprehensive example demonstrating batch publish
   - Shows 3 clients subscribed to different topics
   - Performance comparison between individual and batch operations
   - Demonstrates ~2.6x speed improvement

### Key Design Decisions

1. **Single Lock Acquisition**: The method locks the connection registry once at the start and holds it for all publishes. This is the primary source of performance improvement.

2. **Ordered Results**: Returns results in the same order as the input messages, making it easy to correlate requests with responses.

3. **No Atomicity**: Unlike `subscribe_batch`, this method does not roll back on partial failures. Each publish is independent.

4. **Shared Server Reference**: Changed `run(&self)` instead of `run(self)` to allow the server to be shared via `Arc` while still being able to call `publish_batch`.

### Server API Change

**Important**: The `run()` method signature changed from:
```rust
pub async fn run(self) -> Result<()>
```

To:
```rust
pub async fn run(&self) -> Result<()>
```

This allows the server to be wrapped in `Arc` and shared between the server task and publishing tasks.

**Migration**: Existing code using `server.run()` will continue to work. Code that needs to share the server should wrap it in `Arc`:

```rust
let server = Arc::new(JrowServer::builder().build().await?);

// Clone for server task
let server_clone = Arc::clone(&server);
tokio::spawn(async move {
    server_clone.run().await
});

// Use original for publishing
server.publish_batch(messages).await?;
```

## Testing

### Unit Tests

All existing tests pass:
- 11 tests in `jrow-client`
- 11 tests in `jrow-core`
- 19 tests in `jrow-server`
- Total: 41 tests passing

### Integration Testing

The `examples/publish_batch.rs` example provides integration testing:
- ✅ 3 clients with different subscription patterns
- ✅ Batch publish to 3 topics with correct subscriber counts
- ✅ All clients receive appropriate notifications
- ✅ Performance comparison (10 topics individual vs batch)
- ✅ Demonstrates ~2.6x performance improvement

## Usage Example

```rust
use jrow_server::JrowServer;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build server
    let server = Arc::new(
        JrowServer::builder()
            .bind("127.0.0.1:8080".parse()?)
            .build()
            .await?
    );
    
    // Start server in background
    let server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        server_clone.run().await
    });
    
    // ... clients connect and subscribe ...
    
    // Publish to multiple topics at once
    let messages = vec![
        ("news".to_string(), serde_json::json!({"title": "Breaking"})),
        ("alerts".to_string(), serde_json::json!({"level": "high"})),
        ("updates".to_string(), serde_json::json!({"version": "2.0"})),
    ];
    
    let results = server.publish_batch(messages).await?;
    
    for (topic, count) in results {
        println!("Published to '{}': {} subscribers", topic, count);
    }
    
    Ok(())
}
```

## Compatibility

- **Server-side**: API change to `run(&self)` is backward compatible for most use cases
- **Client-side**: No changes required
- **Protocol**: No protocol changes - uses existing notification mechanism

## Performance Characteristics

### Time Complexity
- **Individual**: O(N * M) where N = topics, M = average subscribers per topic
- **Batch**: O(N * M) but with reduced constant factors due to single lock

### Space Complexity
- **Additional memory**: O(N) for results vector
- **No additional per-message overhead**

### Benchmark Results

From `examples/publish_batch.rs`:

```
Individual publish (10 topics): 60.948µs
Batch publish (10 topics):      23.231µs
Speed improvement: 2.62x faster
```

The improvement comes from:
1. Single lock acquisition (vs N acquisitions)
2. Better CPU cache locality
3. Reduced async overhead

## Future Enhancements

Potential improvements for future versions:

1. **Parallel Notification Sending**: Use `tokio::spawn` to send notifications to subscribers in parallel
2. **Configurable Batch Size**: Add limits to prevent extremely large batches
3. **Error Reporting**: Return detailed error information for failed publishes
4. **Metrics**: Track publish success/failure rates per topic

## Conclusion

The `publish_batch` method provides a significant performance improvement for applications that need to publish to multiple topics simultaneously. The ~2.6x performance improvement, combined with the simple API and backward compatibility, makes it a valuable addition to the toolkit.

