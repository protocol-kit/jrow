# JROW Documentation

This directory contains comprehensive documentation for JROW features and implementations.

## Protocol Specification

- **[SPECIFICATION.md](SPECIFICATION.md)** - **JSON-RPC over WebSocket Protocol Specification**
  - Formal RFC-style specification defining the protocol
  - WebSocket transport requirements and message format
  - Connection lifecycle, request/response patterns, batch processing
  - Error handling and security considerations
  - Pub/Sub and persistent subscriptions extensions
  - Complete with examples, error codes, and implementation checklist

- **[SPECIFICATION-COMPLIANCE.md](SPECIFICATION-COMPLIANCE.md)** - **Specification Compliance Report**
  - ✅ **100% Compliant** - Full verification of implementation against specification
  - Detailed compliance matrix for all sections and requirements
  - Code references and testing coverage
  - Additional features beyond the specification
  - Verification evidence and recommendations

- **[use-cases.md](use-cases.md)** - **Use Cases and Technology Comparison**
  - Real-world use cases where JROW excels
  - Detailed comparison with NATS (messaging system)
  - Detailed comparison with Kafka (event streaming)
  - Decision matrix and decision tree
  - Hybrid architecture patterns
  - Migration strategies from other technologies

## Core Documentation

### Features

- **[persistent-subscriptions.md](persistent-subscriptions.md)** - Persistent subscriptions with exactly-once delivery
  - Reliable message delivery with automatic recovery
  - Persistent storage and state management
  - Retention policies and acknowledgment
  - Complete API reference and examples

- **Persistent Subscriptions Batching** - See [persistent-subscriptions.md](persistent-subscriptions.md) and main [README](../README.md#persistent-subscriptions) for batch operations
  - High-performance batch subscribe, acknowledge, and unsubscribe
  - `subscribe_persistent_batch`, `ack_persistent_batch`, `unsubscribe_persistent_batch`
  - Performance optimization with single network round-trips
  - Examples in [persistent_batch.rs](../examples/persistent_batch.rs)

- **[nats-pattern-matching.md](nats-pattern-matching.md)** - NATS-style pattern matching for persistent subscriptions
  - Token-based pattern matching with wildcards (`*` and `>`)
  - Pattern syntax, validation, and matching algorithm
  - Performance characteristics and best practices
  - Complete examples and use cases

- **[observability.md](observability.md)** - OpenTelemetry observability guide
  - Distributed tracing, metrics, and structured logging
  - Jaeger, Prometheus, and Grafana integration
  - Configuration and usage examples

- **[middleware.md](middleware.md)** - Middleware system documentation
  - Synchronous and asynchronous middleware
  - Pre/post request hooks
  - Built-in and custom middleware examples

- **[reconnection.md](reconnection.md)** - Automatic reconnection guide
  - Reconnection strategies (exponential backoff, fixed delay)
  - Configuration and usage
  - Connection state management

### Implementation Details

- **[batch-implementation.md](batch-implementation.md)** - Batch requests implementation
- **[batch-subscribe-implementation.md](batch-subscribe-implementation.md)** - Batch subscribe/unsubscribe
- **[publish-batch-implementation.md](publish-batch-implementation.md)** - Batch publishing
- **[pubsub-implementation.md](pubsub-implementation.md)** - Pub/sub system implementation
- **[opentelemetry-implementation.md](opentelemetry-implementation.md)** - OpenTelemetry integration details
- **[asyncapi-template-implementation.md](asyncapi-template-implementation.md)** - AsyncAPI templates
- **[deploy-script-template.md](deploy-script-template.md)** - Deployment script templates
- **[implementation-summary.md](implementation-summary.md)** - Overall implementation summary

## Quick Links

- [Main README](../README.md) - Project overview and quick start
- [Quick Start Guide](../QUICKSTART.md) - Get started quickly
- [Examples](../examples/) - Working code examples
- [Templates](../templates/README.md) - Project templates and deployment configs

## Documentation Index

### By Feature

| Feature | User Guide | Implementation Details |
|---------|------------|------------------------|
| **Protocol Specification** | **[SPECIFICATION.md](SPECIFICATION.md)** | **Formal RFC-style spec** |
| **Use Cases & Comparison** | **[use-cases.md](use-cases.md)** | **vs NATS, Kafka** |
| Persistent Subscriptions | [persistent-subscriptions.md](persistent-subscriptions.md) | Built-in |
| Persistent Subscriptions Batching | [README](../README.md#persistent-subscriptions) / [Example](../examples/persistent_batch.rs) | Built-in |
| NATS Pattern Matching | [nats-pattern-matching.md](nats-pattern-matching.md) | Built-in |
| Batch Requests | [README](../README.md#batch-requests) | [batch-implementation.md](batch-implementation.md) |
| Pub/Sub | [README](../README.md#publishsubscribe-pubsub) | [pubsub-implementation.md](pubsub-implementation.md) |
| Middleware | [middleware.md](middleware.md) | Built-in |
| Reconnection | [reconnection.md](reconnection.md) | Built-in |
| Observability | [observability.md](observability.md) | [opentelemetry-implementation.md](opentelemetry-implementation.md) |
| AsyncAPI | [Templates README](../templates/README.md) | [asyncapi-template-implementation.md](asyncapi-template-implementation.md) |

### By Use Case

- **Choosing Technology**: [use-cases.md](use-cases.md) - JROW vs NATS vs Kafka comparison
- **Protocol Reference**: [SPECIFICATION.md](SPECIFICATION.md) - Formal protocol specification
- **Getting Started**: [QUICKSTART.md](../QUICKSTART.md)
- **Production Deployment**: [observability.md](observability.md), [Templates](../templates/README.md)
- **Advanced Features**: [middleware.md](middleware.md), [reconnection.md](reconnection.md)
- **API Documentation**: [asyncapi-template-implementation.md](asyncapi-template-implementation.md)

## Contributing

When adding new documentation:
1. Use lowercase kebab-case for filenames (e.g., `my-feature.md`)
2. Add an entry to this README
3. Update links in related documents
4. Include code examples where applicable

