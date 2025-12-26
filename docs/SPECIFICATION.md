# JSON-RPC over WebSocket Specification

**Version:** 1.0  
**Status:** Draft  
**Last Updated:** December 26, 2025

## Abstract

This specification defines how JSON-RPC 2.0 protocol messages are transported over WebSocket connections. It extends the base JSON-RPC 2.0 specification with WebSocket-specific transport mechanisms, connection management, and additional patterns for publish/subscribe and persistent subscriptions.

## Table of Contents

1. [Introduction](#1-introduction)
2. [Conventions](#2-conventions)
3. [Transport Layer](#3-transport-layer)
4. [Message Format](#4-message-format)
5. [Connection Lifecycle](#5-connection-lifecycle)
6. [Request-Response Pattern](#6-request-response-pattern)
7. [Notifications](#7-notifications)
8. [Batch Requests](#8-batch-requests)
9. [Error Handling](#9-error-handling)
10. [Publish/Subscribe Extension](#10-publishsubscribe-extension)
11. [Persistent Subscriptions Extension](#11-persistent-subscriptions-extension)
12. [Security Considerations](#12-security-considerations)
13. [Implementation Guidelines](#13-implementation-guidelines)
14. [References](#14-references)

---

## 1. Introduction

### 1.1 Purpose

This specification defines a standardized method for implementing JSON-RPC 2.0 over WebSocket transport. While JSON-RPC 2.0 is transport-agnostic, this specification addresses WebSocket-specific concerns including connection management, bidirectional communication, and real-time messaging patterns.

### 1.2 Scope

This specification covers:

- WebSocket transport requirements for JSON-RPC 2.0
- Connection establishment and lifecycle management
- Message framing and encoding
- Bidirectional communication patterns
- Optional extensions for publish/subscribe and persistent messaging

This specification does NOT cover:

- Application-level authentication and authorization mechanisms
- WebSocket subprotocol negotiation details
- Transport layer security (TLS/SSL) configuration

### 1.3 Terminology

- **Client**: An endpoint that initiates a WebSocket connection
- **Server**: An endpoint that accepts WebSocket connections
- **Endpoint**: Either a client or server
- **Connection**: An established WebSocket connection between two endpoints
- **Message**: A WebSocket text frame containing a JSON-RPC payload
- **Method**: A named RPC procedure that can be invoked
- **Handler**: Server-side code that processes a method invocation

---

## 2. Conventions

### 2.1 Requirement Levels

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

### 2.2 JSON Syntax

This specification uses JSON syntax as defined in [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259). All examples use JSON notation.

### 2.3 WebSocket Protocol

This specification assumes WebSocket connections conform to [RFC 6455](https://www.rfc-editor.org/rfc/rfc6455).

---

## 3. Transport Layer

### 3.1 WebSocket Requirements

#### 3.1.1 Protocol Version

Implementations MUST support WebSocket protocol version 13 as defined in RFC 6455.

#### 3.1.2 Message Type

JSON-RPC messages MUST be sent as WebSocket **text frames** (opcode 0x1), NOT binary frames.

#### 3.1.3 Message Encoding

Messages MUST be encoded as UTF-8 text. The WebSocket text frame MUST contain a valid JSON document.

#### 3.1.4 Frame Size

Implementations SHOULD support messages up to at least 65,536 bytes (64 KB). Implementations MAY impose larger or smaller limits but MUST document these limits.

If a message exceeds the implementation's size limit, the implementation SHOULD return a JSON-RPC error with code `-32600` (Invalid Request) before closing the connection.

### 3.2 Connection Establishment

#### 3.2.1 WebSocket Handshake

Clients MUST initiate connections using standard WebSocket handshake:

```http
GET / HTTP/1.1
Host: example.com:8080
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
```

Servers MUST respond with:

```http
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
```

#### 3.2.2 Subprotocol Negotiation

This specification does NOT require WebSocket subprotocol negotiation. Implementations MAY use the subprotocol `jsonrpc` for explicit signaling:

```http
Sec-WebSocket-Protocol: jsonrpc
```

#### 3.2.3 Connection State

Upon successful handshake completion, the connection enters the **OPEN** state and both endpoints MAY send JSON-RPC messages.

### 3.3 Keep-Alive

#### 3.3.1 WebSocket Ping/Pong

Implementations SHOULD use WebSocket Ping/Pong frames (opcodes 0x9/0xA) to detect connection failures.

Servers SHOULD send Ping frames at regular intervals (RECOMMENDED: 30-60 seconds).

Endpoints MUST respond to Ping frames with Pong frames as specified in RFC 6455.

#### 3.3.2 Application-Level Heartbeat

Implementations MAY implement application-level heartbeat using JSON-RPC notifications:

**Client to Server:**
```json
{
  "jsonrpc": "2.0",
  "method": "ping"
}
```

**Server to Client:**
```json
{
  "jsonrpc": "2.0",
  "method": "pong"
}
```

These are notifications (no `id` field) and do not require responses.

### 3.4 Connection Termination

#### 3.4.1 Clean Shutdown

Endpoints SHOULD perform a clean shutdown using WebSocket Close frames (opcode 0x8).

Typical close codes:
- `1000`: Normal closure
- `1001`: Going away (endpoint shutting down)
- `1002`: Protocol error
- `1003`: Unsupported data (e.g., binary frame received)

#### 3.4.2 Abrupt Disconnection

Implementations MUST handle abrupt disconnections (network failure, process termination) gracefully.

Pending requests MAY be canceled or retried based on implementation policy.

---

## 4. Message Format

### 4.1 JSON-RPC 2.0 Base

All messages MUST conform to [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification).

### 4.2 Request Object

A JSON-RPC request sent over WebSocket:

```json
{
  "jsonrpc": "2.0",
  "method": "subtract",
  "params": {"minuend": 42, "subtrahend": 23},
  "id": 1
}
```

**Required members:**
- `jsonrpc`: String, MUST be exactly "2.0"
- `method`: String, name of the method to invoke
- `id`: String, Number, or Null, request identifier

**Optional members:**
- `params`: Structured value (Object or Array), method parameters

### 4.3 Response Object

A successful JSON-RPC response:

```json
{
  "jsonrpc": "2.0",
  "result": 19,
  "id": 1
}
```

An error response:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Method not found"
  },
  "id": 1
}
```

**Required members:**
- `jsonrpc`: String, MUST be exactly "2.0"
- `result`: Any JSON value (MUST be present on success)
- `error`: Error object (MUST be present on error)
- `id`: MUST match the request `id`

### 4.4 Notification Object

A notification is a request without an `id`:

```json
{
  "jsonrpc": "2.0",
  "method": "update",
  "params": {"status": "processing"}
}
```

Notifications MUST NOT receive responses.

### 4.5 Batch Messages

Multiple messages MAY be sent in a single WebSocket frame as a JSON array:

```json
[
  {"jsonrpc": "2.0", "method": "sum", "params": [1, 2, 4], "id": "1"},
  {"jsonrpc": "2.0", "method": "subtract", "params": [42, 23], "id": "2"},
  {"jsonrpc": "2.0", "method": "notify_hello", "params": [7]}
]
```

Response:

```json
[
  {"jsonrpc": "2.0", "result": 7, "id": "1"},
  {"jsonrpc": "2.0", "result": 19, "id": "2"}
]
```

Note: Notification in the batch does not produce a response.

---

## 5. Connection Lifecycle

### 5.1 States

A WebSocket connection used for JSON-RPC MUST maintain the following states:

1. **CONNECTING**: WebSocket handshake in progress
2. **OPEN**: Connection established, ready for messaging
3. **CLOSING**: Close handshake initiated
4. **CLOSED**: Connection terminated

### 5.2 State Transitions

```
CONNECTING → OPEN → CLOSING → CLOSED
     ↓                 ↓
   CLOSED           CLOSED
```

### 5.3 State Behaviors

#### CONNECTING
- No JSON-RPC messages may be sent
- Handshake must complete or fail

#### OPEN
- JSON-RPC messages may be sent and received
- Both endpoints may initiate requests or send notifications
- Either endpoint may initiate connection close

#### CLOSING
- No new JSON-RPC messages should be sent
- Pending requests may be canceled
- Close handshake in progress

#### CLOSED
- No messages can be sent or received
- All resources should be released

---

## 6. Request-Response Pattern

### 6.1 Request Initiation

Either endpoint (client or server) MAY initiate a request at any time while the connection is in the OPEN state.

### 6.2 Request ID

Each request MUST include a unique `id` within the scope of the connection.

The `id` value:
- MUST be a String, Number, or Null
- SHOULD be unique for pending requests from the same sender
- MUST be echoed in the corresponding response

RECOMMENDED: Use incrementing integers or UUIDs for request IDs.

### 6.3 Response Matching

The receiving endpoint MUST match responses to requests using the `id` field.

If a response is received with an unknown `id`, the endpoint:
- SHOULD log a warning
- MUST ignore the response
- MAY close the connection if this occurs repeatedly

### 6.4 Request Timeout

Implementations SHOULD implement request timeouts to detect failed or unresponsive peers.

RECOMMENDED timeout: 30-60 seconds for typical requests.

If a request times out:
- The sender SHOULD clean up request state
- The sender MAY close the connection
- If a late response arrives, it SHOULD be ignored

### 6.5 Method Invocation

When a request is received:

1. Validate JSON-RPC structure (return `-32600` if invalid)
2. Lookup method handler (return `-32601` if not found)
3. Validate parameters (return `-32602` if invalid)
4. Execute handler
5. Return result or error

### 6.6 Bidirectional RPC

Both clients and servers MAY act as RPC requesters and responders:

**Client to Server:**
```json
→ {"jsonrpc": "2.0", "method": "getUser", "params": {"id": 123}, "id": 1}
← {"jsonrpc": "2.0", "result": {"id": 123, "name": "Alice"}, "id": 1}
```

**Server to Client:**
```json
← {"jsonrpc": "2.0", "method": "refresh", "params": {}, "id": "srv-1"}
→ {"jsonrpc": "2.0", "result": "ok", "id": "srv-1"}
```

---

## 7. Notifications

### 7.1 Notification Semantics

Notifications are **fire-and-forget** messages. The sender MUST NOT expect a response.

### 7.2 Notification Object

A notification is a request object WITHOUT an `id` field:

```json
{
  "jsonrpc": "2.0",
  "method": "statusUpdate",
  "params": {"status": "completed", "progress": 100}
}
```

### 7.3 No Response

The receiver MUST NOT send a response to a notification, even if an error occurs during processing.

### 7.4 Error Handling

If a notification cannot be processed:
- The receiver SHOULD log the error
- The receiver MUST NOT send an error response
- The connection SHOULD remain open

### 7.5 Bidirectional Notifications

Both endpoints MAY send notifications:

**Client to Server:**
```json
→ {"jsonrpc": "2.0", "method": "userTyping", "params": {"user": "alice"}}
```

**Server to Client:**
```json
← {"jsonrpc": "2.0", "method": "newMessage", "params": {"from": "bob", "text": "Hi"}}
```

---

## 8. Batch Requests

### 8.1 Batch Format

Multiple JSON-RPC messages MAY be sent together as a JSON array:

```json
[
  {"jsonrpc": "2.0", "method": "sum", "params": [1, 2, 4], "id": "1"},
  {"jsonrpc": "2.0", "method": "notify_hello", "params": [7]},
  {"jsonrpc": "2.0", "method": "subtract", "params": [42, 23], "id": "2"},
  {"jsonrpc": "2.0", "method": "get_data", "id": "9"}
]
```

### 8.2 Batch Processing

The receiver:
- MUST process all valid requests in the batch
- SHOULD process requests concurrently (unless order is required by application semantics)
- MUST return responses for all requests (not notifications)
- MAY return responses in any order

### 8.3 Batch Response

Responses are returned as a JSON array:

```json
[
  {"jsonrpc": "2.0", "result": 7, "id": "1"},
  {"jsonrpc": "2.0", "result": 19, "id": "2"},
  {"jsonrpc": "2.0", "result": ["hello", 5], "id": "9"}
]
```

Note: The notification `notify_hello` does not produce a response.

### 8.4 Empty Batch

An empty array `[]` is invalid. The receiver MUST return:

```json
{
  "jsonrpc": "2.0",
  "error": {"code": -32600, "message": "Invalid Request"},
  "id": null
}
```

### 8.5 Batch Size Limits

Implementations SHOULD limit batch size to prevent resource exhaustion.

RECOMMENDED maximum: 100 requests per batch.

If batch size exceeds the limit:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": "Batch size exceeds maximum of 100"
  },
  "id": null
}
```

### 8.6 Partial Batch Failure

If some requests in a batch are invalid, implementations SHOULD:
- Return error responses for invalid requests
- Process valid requests normally
- NOT reject the entire batch

---

## 9. Error Handling

### 9.1 Standard Error Codes

JSON-RPC 2.0 defines these standard error codes:

| Code | Message | Meaning |
|------|---------|---------|
| -32700 | Parse error | Invalid JSON was received |
| -32600 | Invalid Request | JSON is not a valid Request object |
| -32601 | Method not found | Method does not exist |
| -32602 | Invalid params | Invalid method parameters |
| -32603 | Internal error | Internal JSON-RPC error |

### 9.2 Error Object Structure

```json
{
  "code": -32601,
  "message": "Method not found",
  "data": "No handler registered for method 'calculateX'"
}
```

**Required:**
- `code`: Integer error code
- `message`: String, short description

**Optional:**
- `data`: Additional information (any JSON type)

### 9.3 Application Error Codes

Applications MAY define custom error codes. These SHOULD be in the range:
- `-32099` to `-32000`: Reserved for implementation-defined server errors
- `-32000` and below: Available for application use

Example:

```json
{
  "code": -32001,
  "message": "Insufficient balance",
  "data": {"balance": 10, "required": 50}
}
```

### 9.4 Transport Errors

WebSocket transport errors are handled separately from JSON-RPC errors:

**Parse Error**: If invalid JSON is received:
```json
{
  "jsonrpc": "2.0",
  "error": {"code": -32700, "message": "Parse error"},
  "id": null
}
```

**Connection Errors**: Network or WebSocket protocol errors SHOULD close the connection with an appropriate WebSocket close code.

---

## 10. Publish/Subscribe Extension

### 10.1 Overview

This extension enables topic-based publish/subscribe messaging over JSON-RPC/WebSocket.

This extension is OPTIONAL. Implementations MAY support pub/sub without implementing this exact interface.

### 10.2 Built-in Methods

#### 10.2.1 rpc.subscribe

Subscribe to a topic:

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.subscribe",
  "params": {"topic": "chat.messages"},
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {"subscribed": true},
  "id": 1
}
```

#### 10.2.2 rpc.unsubscribe

Unsubscribe from a topic:

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.unsubscribe",
  "params": {"topic": "chat.messages"},
  "id": 2
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {"unsubscribed": true},
  "id": 2
}
```

### 10.3 Topic Notifications

When a message is published to a topic, all subscribers receive a notification:

```json
{
  "jsonrpc": "2.0",
  "method": "rpc.notification",
  "params": {
    "topic": "chat.messages",
    "data": {
      "from": "alice",
      "message": "Hello everyone!"
    }
  }
}
```

### 10.4 Topic Naming

Topics SHOULD use dot-separated naming:
- `chat.messages`
- `stock.prices.AAPL`
- `events.user.login`

### 10.5 Pattern Matching (Optional)

Implementations MAY support wildcard subscriptions using NATS-style patterns:

**Single-level wildcard** (`*`):
- `events.*` matches `events.user` and `events.admin`
- `events.*` does NOT match `events.user.login`

**Multi-level wildcard** (`>`):
- `events.>` matches `events.user`, `events.user.login`, etc.
- `>` MUST be the last token

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.subscribe",
  "params": {"topic": "stock.prices.*"},
  "id": 3
}
```

### 10.6 Subscription Lifecycle

- Subscriptions are connection-scoped
- When a connection closes, all subscriptions MUST be removed
- Subscribers SHOULD unsubscribe explicitly before disconnecting

### 10.7 Batch Subscribe/Unsubscribe (Optional)

#### 10.7.1 Batch Subscribe

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.subscribe.batch",
  "params": {
    "topics": ["news", "alerts", "updates"]
  },
  "id": 4
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "subscribed": ["news", "alerts", "updates"]
  },
  "id": 4
}
```

#### 10.7.2 Batch Unsubscribe

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.unsubscribe.batch",
  "params": {
    "topics": ["news", "alerts"]
  },
  "id": 5
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "unsubscribed": ["news", "alerts"]
  },
  "id": 5
}
```

---

## 11. Persistent Subscriptions Extension

### 11.1 Overview

Persistent subscriptions provide reliable, exactly-once message delivery with durable storage and automatic recovery.

This extension is OPTIONAL.

### 11.2 Characteristics

- Messages are stored in a durable database
- Each subscriber has a unique subscription ID
- Messages require explicit acknowledgment
- Unacknowledged messages are redelivered
- Automatic resume from last acknowledged position

### 11.3 Built-in Methods

#### 11.3.1 rpc.subscribe.persistent

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.subscribe.persistent",
  "params": {
    "subscription_id": "order-processor-1",
    "topic": "orders"
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "subscription_id": "order-processor-1",
    "topic": "orders",
    "resumed_from_sequence": 42
  },
  "id": 1
}
```

#### 11.3.2 rpc.acknowledge.persistent

Acknowledge message receipt:

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.acknowledge.persistent",
  "params": {
    "subscription_id": "order-processor-1",
    "sequence_id": 43
  },
  "id": 2
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {"acknowledged": true},
  "id": 2
}
```

#### 11.3.3 rpc.unsubscribe.persistent

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.unsubscribe.persistent",
  "params": {
    "subscription_id": "order-processor-1"
  },
  "id": 3
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {"unsubscribed": true},
  "id": 3
}
```

### 11.4 Message Delivery

Persistent messages include metadata:

```json
{
  "jsonrpc": "2.0",
  "method": "rpc.notification.persistent",
  "params": {
    "subscription_id": "order-processor-1",
    "topic": "orders",
    "sequence_id": 43,
    "timestamp": "2025-12-26T12:00:00Z",
    "data": {
      "order_id": "ORD-12345",
      "status": "confirmed"
    }
  }
}
```

**Fields:**
- `subscription_id`: Unique subscription identifier
- `topic`: Topic name
- `sequence_id`: Monotonically increasing message sequence number
- `timestamp`: ISO 8601 timestamp
- `data`: Application payload

### 11.5 Acknowledgment Semantics

- Messages MUST be acknowledged explicitly
- Unacknowledged messages MUST be redelivered on reconnection
- Implementations SHOULD support a timeout for unacknowledged messages
- Only one connection per subscription ID should be active at a time

### 11.6 Retention Policies

Implementations SHOULD support configurable retention policies:

**Time-based:**
- Delete messages older than specified duration

**Count-based:**
- Keep only the last N messages

**Size-based:**
- Limit total storage size

**Example configuration (implementation-specific):**
```json
{
  "topic": "orders",
  "retention": {
    "max_age_seconds": 86400,
    "max_count": 10000,
    "max_bytes": 10485760
  }
}
```

### 11.7 Batch Operations (Optional)

#### 11.7.1 Batch Persistent Subscribe

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.subscribe.persistent.batch",
  "params": {
    "subscriptions": [
      {"subscription_id": "order-proc", "topic": "orders"},
      {"subscription_id": "payment-proc", "topic": "payments"}
    ]
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "subscriptions": [
      {"subscription_id": "order-proc", "resumed_from_sequence": 10},
      {"subscription_id": "payment-proc", "resumed_from_sequence": 5}
    ]
  },
  "id": 1
}
```

#### 11.7.2 Batch Acknowledge

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.acknowledge.persistent.batch",
  "params": {
    "acknowledgments": [
      {"subscription_id": "order-proc", "sequence_id": 11},
      {"subscription_id": "order-proc", "sequence_id": 12},
      {"subscription_id": "payment-proc", "sequence_id": 6}
    ]
  },
  "id": 2
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "acknowledged": [
      {"subscription_id": "order-proc", "sequence_id": 11, "success": true},
      {"subscription_id": "order-proc", "sequence_id": 12, "success": true},
      {"subscription_id": "payment-proc", "sequence_id": 6, "success": true}
    ]
  },
  "id": 2
}
```

---

## 12. Security Considerations

### 12.1 Transport Security

Implementations SHOULD use `wss://` (WebSocket Secure) for all production deployments.

TLS 1.2 or higher MUST be used when using secure WebSockets.

### 12.2 Authentication

This specification does NOT define authentication mechanisms. Implementations SHOULD implement one of:

1. **Token-based authentication**: Pass bearer token in WebSocket handshake headers
2. **Cookie-based authentication**: Use session cookies
3. **Message-level authentication**: First message contains credentials
4. **Certificate-based authentication**: Use client TLS certificates

### 12.3 Authorization

Method handlers SHOULD verify authorization before executing operations.

Example error for unauthorized access:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Unauthorized",
    "data": "Insufficient permissions to call this method"
  },
  "id": 1
}
```

### 12.4 Input Validation

Implementations MUST validate all input parameters before processing.

Untrusted input SHOULD be sanitized before use in:
- Database queries
- File system operations
- System commands
- HTML output

### 12.5 Rate Limiting

Implementations SHOULD implement rate limiting to prevent abuse:

- Limit requests per connection per time window
- Limit batch sizes
- Limit message sizes
- Limit subscription counts per connection

### 12.6 Denial of Service Prevention

Protect against DoS attacks:

1. **Connection limits**: Maximum concurrent connections
2. **Message size limits**: Maximum message size
3. **Batch size limits**: Maximum requests per batch
4. **Timeout enforcement**: Request timeouts
5. **Resource quotas**: Per-connection memory/CPU limits

### 12.7 Data Validation

All JSON-RPC messages MUST be validated:

1. Valid UTF-8 encoding
2. Valid JSON syntax
3. Valid JSON-RPC structure
4. Type checking for parameters

Invalid messages SHOULD result in appropriate error responses, not connection termination (unless repeated).

---

## 13. Implementation Guidelines

### 13.1 Connection Management

Implementations SHOULD:
- Support connection pooling for clients
- Track connection state accurately
- Clean up resources on disconnect
- Implement graceful shutdown

### 13.2 Concurrency

Implementations SHOULD:
- Handle requests concurrently when possible
- Use appropriate synchronization for shared state
- Avoid blocking operations in message handlers
- Implement timeouts for all operations

### 13.3 Error Handling

Implementations SHOULD:
- Log all errors with sufficient context
- Return appropriate JSON-RPC error codes
- Avoid exposing sensitive information in error messages
- Implement error monitoring and alerting

### 13.4 Performance

Implementations SHOULD optimize for:
- Low latency message processing
- High throughput (messages/second)
- Efficient memory usage
- Minimal CPU overhead

### 13.5 Observability

Implementations SHOULD provide:
- Metrics: connection count, request rate, error rate
- Tracing: request/response correlation
- Logging: structured logs with correlation IDs
- Health checks: endpoint for service health

### 13.6 Testing

Implementations SHOULD test:
- Valid and invalid JSON-RPC messages
- Connection lifecycle events
- Concurrent request handling
- Error conditions
- Reconnection scenarios
- Resource limits

---

## 14. References

### 14.1 Normative References

- **[RFC 2119]** Key words for use in RFCs to Indicate Requirement Levels  
  https://www.rfc-editor.org/rfc/rfc2119

- **[RFC 6455]** The WebSocket Protocol  
  https://www.rfc-editor.org/rfc/rfc6455

- **[RFC 8259]** The JavaScript Object Notation (JSON) Data Interchange Format  
  https://www.rfc-editor.org/rfc/rfc8259

- **[JSON-RPC 2.0]** JSON-RPC 2.0 Specification  
  https://www.jsonrpc.org/specification

### 14.2 Informative References

- **[RFC 7235]** HTTP Authentication  
  https://www.rfc-editor.org/rfc/rfc7235

- **[RFC 8446]** The Transport Layer Security (TLS) Protocol Version 1.3  
  https://www.rfc-editor.org/rfc/rfc8446

- **[NATS]** NATS Messaging Subject-Based Messaging  
  https://docs.nats.io/nats-concepts/subjects

---

## Appendix A: Complete Examples

### A.1 Simple Request-Response

**Client request:**
```json
{
  "jsonrpc": "2.0",
  "method": "getUserProfile",
  "params": {"userId": "12345"},
  "id": 1
}
```

**Server response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "userId": "12345",
    "name": "Alice Johnson",
    "email": "alice@example.com"
  },
  "id": 1
}
```

### A.2 Notification

**Client notification:**
```json
{
  "jsonrpc": "2.0",
  "method": "userTyping",
  "params": {"userId": "12345", "typing": true}
}
```

No response is sent.

### A.3 Batch Request

**Client batch request:**
```json
[
  {
    "jsonrpc": "2.0",
    "method": "getUserProfile",
    "params": {"userId": "12345"},
    "id": 1
  },
  {
    "jsonrpc": "2.0",
    "method": "getUserPosts",
    "params": {"userId": "12345", "limit": 10},
    "id": 2
  },
  {
    "jsonrpc": "2.0",
    "method": "logActivity",
    "params": {"action": "profile_view"}
  }
]
```

**Server batch response:**
```json
[
  {
    "jsonrpc": "2.0",
    "result": {
      "userId": "12345",
      "name": "Alice Johnson"
    },
    "id": 1
  },
  {
    "jsonrpc": "2.0",
    "result": {
      "posts": [
        {"id": "p1", "title": "Hello World"},
        {"id": "p2", "title": "JSON-RPC Tutorial"}
      ]
    },
    "id": 2
  }
]
```

### A.4 Error Response

**Client request:**
```json
{
  "jsonrpc": "2.0",
  "method": "transferFunds",
  "params": {"from": "12345", "to": "67890", "amount": 100},
  "id": 3
}
```

**Server error response:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Insufficient funds",
    "data": {
      "available": 50,
      "requested": 100
    }
  },
  "id": 3
}
```

### A.5 Pub/Sub Flow

**Client subscribes:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.subscribe",
  "params": {"topic": "stock.prices.AAPL"},
  "id": 1
}
```

**Server confirms:**
```json
{
  "jsonrpc": "2.0",
  "result": {"subscribed": true},
  "id": 1
}
```

**Server publishes:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.notification",
  "params": {
    "topic": "stock.prices.AAPL",
    "data": {
      "symbol": "AAPL",
      "price": 150.25,
      "timestamp": "2025-12-26T12:00:00Z"
    }
  }
}
```

**Client unsubscribes:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.unsubscribe",
  "params": {"topic": "stock.prices.AAPL"},
  "id": 2
}
```

**Server confirms:**
```json
{
  "jsonrpc": "2.0",
  "result": {"unsubscribed": true},
  "id": 2
}
```

### A.6 Persistent Subscription Flow

**Client subscribes:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.subscribe.persistent",
  "params": {
    "subscription_id": "order-processor-1",
    "topic": "orders"
  },
  "id": 1
}
```

**Server response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "subscription_id": "order-processor-1",
    "topic": "orders",
    "resumed_from_sequence": 0
  },
  "id": 1
}
```

**Server delivers message:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.notification.persistent",
  "params": {
    "subscription_id": "order-processor-1",
    "topic": "orders",
    "sequence_id": 1,
    "timestamp": "2025-12-26T12:00:00Z",
    "data": {
      "order_id": "ORD-001",
      "status": "confirmed"
    }
  }
}
```

**Client acknowledges:**
```json
{
  "jsonrpc": "2.0",
  "method": "rpc.acknowledge.persistent",
  "params": {
    "subscription_id": "order-processor-1",
    "sequence_id": 1
  },
  "id": 2
}
```

**Server confirms:**
```json
{
  "jsonrpc": "2.0",
  "result": {"acknowledged": true},
  "id": 2
}
```

---

## Appendix B: WebSocket Frame Details

### B.1 Text Frame Structure

All JSON-RPC messages use WebSocket text frames (opcode 0x1):

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-------+-+-------------+-------------------------------+
|F|R|R|R| opcode|M| Payload len |    Extended payload length    |
|I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
|N|V|V|V|       |S|             |   (if payload len==126/127)   |
| |1|2|3|       |K|             |                               |
+-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
|     Extended payload length continued, if payload len == 127  |
+ - - - - - - - - - - - - - - - +-------------------------------+
|                               |Masking-key, if MASK set to 1  |
+-------------------------------+-------------------------------+
| Masking-key (continued)       |          Payload Data         |
+-------------------------------- - - - - - - - - - - - - - - - +
:                     Payload Data continued ...                :
+ - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
|                     Payload Data continued ...                |
+---------------------------------------------------------------+
```

**For JSON-RPC:**
- FIN = 1 (final frame)
- opcode = 0x1 (text frame)
- Payload = UTF-8 encoded JSON

### B.2 Close Frame

Close frame (opcode 0x8) with status code:

```
+--------+--------+-------- ... --------+
| Status |   Reason Phrase (optional)   |
| (2)    |          (variable)          |
+--------+--------+-------- ... --------+
```

Common status codes:
- 1000: Normal closure
- 1001: Going away
- 1002: Protocol error

---

## Appendix C: Error Code Registry

### C.1 Standard JSON-RPC Errors

| Code | Message | Description |
|------|---------|-------------|
| -32700 | Parse error | Invalid JSON received |
| -32600 | Invalid Request | Not a valid Request object |
| -32601 | Method not found | Method does not exist |
| -32602 | Invalid params | Invalid method parameters |
| -32603 | Internal error | Internal JSON-RPC error |

### C.2 Reserved Range

| Range | Usage |
|-------|-------|
| -32768 to -32000 | Reserved for pre-defined errors |
| -32000 to -32099 | Server error (implementation defined) |

### C.3 Suggested Application Error Codes

| Code | Suggested Use |
|------|---------------|
| -32001 | Unauthorized |
| -32002 | Forbidden |
| -32003 | Not found |
| -32004 | Validation error |
| -32005 | Conflict |
| -32006 | Rate limit exceeded |
| -32007 | Resource exhausted |
| -32008 | Timeout |
| -32009 | Service unavailable |

---

## Appendix D: Implementation Checklist

### D.1 Core Requirements

- [ ] WebSocket protocol version 13 support
- [ ] UTF-8 text frame encoding
- [ ] JSON-RPC 2.0 message format
- [ ] Request-response pattern
- [ ] Notification support
- [ ] Batch request support
- [ ] Standard error codes
- [ ] Connection lifecycle management
- [ ] Clean shutdown

### D.2 Recommended Features

- [ ] Request timeout
- [ ] Keep-alive (ping/pong)
- [ ] Graceful error handling
- [ ] Connection state tracking
- [ ] Logging and metrics
- [ ] Rate limiting
- [ ] TLS/SSL support (wss://)

### D.3 Optional Extensions

- [ ] Bidirectional RPC
- [ ] Publish/Subscribe
- [ ] Pattern-based subscriptions
- [ ] Persistent subscriptions
- [ ] Batch subscribe/unsubscribe
- [ ] Automatic reconnection
- [ ] Middleware/interceptors
- [ ] OpenTelemetry integration

---

## Appendix E: Change Log

### Version 1.0 (Draft) - December 26, 2025

- Initial specification
- Core JSON-RPC over WebSocket protocol
- Publish/Subscribe extension
- Persistent Subscriptions extension
- Security considerations
- Implementation guidelines

---

**End of Specification**

