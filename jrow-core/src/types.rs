//! JSON-RPC 2.0 types as defined in the specification
//!
//! This module implements all core data structures from the JSON-RPC 2.0 specification
//! (https://www.jsonrpc.org/specification). These types are designed to be:
//!
//! - **Spec-compliant**: Strict adherence to JSON-RPC 2.0 requirements
//! - **Type-safe**: Rust's type system prevents invalid message construction
//! - **Serializable**: Full serde support for JSON encoding/decoding
//!
//! # Message Types
//!
//! JSON-RPC 2.0 defines three primary message types:
//!
//! 1. **Request**: A call to a remote method that expects a response
//! 2. **Notification**: A call to a remote method with no response expected
//! 3. **Response**: The result of processing a request (success or error)
//!
//! # Request IDs
//!
//! Request IDs are used to correlate requests with responses. The spec allows
//! string, number, or null IDs, though null is discouraged as it makes correlation
//! difficult in practice.

use crate::error::JsonRpcErrorData;
use serde::{Deserialize, Serialize};
use std::fmt;

/// JSON-RPC 2.0 request ID
///
/// The request identifier is used to correlate a request with its corresponding
/// response. According to the spec, an ID can be a string, number, or null.
///
/// # Why Multiple Types?
///
/// Different clients may prefer different ID schemes:
/// - **String IDs**: Useful for UUIDs or human-readable identifiers
/// - **Number IDs**: Simple sequential counters, memory efficient
/// - **Null IDs**: Technically allowed but not recommended (makes correlation impossible)
///
/// # Implementation Notes
///
/// This enum uses `#[serde(untagged)]` to serialize directly as the inner value
/// without a type discriminator, matching the JSON-RPC 2.0 spec exactly.
///
/// The type implements `Hash` and `Eq` to enable using IDs as HashMap keys,
/// which is useful for tracking pending requests.
///
/// # Examples
///
/// ```rust
/// use jrow_core::Id;
///
/// // From a string
/// let id1: Id = "req-123".into();
///
/// // From a number (i64)
/// let id2: Id = 42i64.into();
///
/// // Display formatting
/// assert_eq!(id1.to_string(), "\"req-123\"");
/// assert_eq!(id2.to_string(), "42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Id {
    /// String identifier - useful for UUIDs or correlation tokens
    String(String),
    /// Numeric identifier - efficient for sequential request counters
    Number(i64),
    /// Null identifier - allowed by spec but not recommended
    /// (makes request/response correlation impossible)
    Null,
}

impl fmt::Display for Id {
    /// Format the ID for human-readable display
    ///
    /// This formats IDs in a JSON-like representation:
    /// - Strings are quoted
    /// - Numbers are displayed as-is
    /// - Null is displayed as "null"
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Id::String(s) => write!(f, "\"{}\"", s),
            Id::Number(n) => write!(f, "{}", n),
            Id::Null => write!(f, "null"),
        }
    }
}

// Convenience conversions to make ID creation ergonomic
// These allow using `.into()` or passing values directly where `Id` is expected

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id::String(s)
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id::String(s.to_string())
    }
}

impl From<i64> for Id {
    fn from(n: i64) -> Self {
        Id::Number(n)
    }
}

impl From<u64> for Id {
    /// Convert from u64 to Id
    ///
    /// Note: This casts to i64, so values > i64::MAX will wrap around.
    /// Most applications use much smaller ID values, so this is rarely an issue.
    fn from(n: u64) -> Self {
        Id::Number(n as i64)
    }
}

/// JSON-RPC 2.0 request message
///
/// A request represents a call to a remote method that expects a response.
/// The response will have a matching `id` field to correlate with this request.
///
/// # Spec Compliance
///
/// According to JSON-RPC 2.0 spec, a request MUST contain:
/// - `jsonrpc`: Must be exactly "2.0"
/// - `method`: The name of the method to invoke
/// - `id`: An identifier to correlate with the response
///
/// And MAY contain:
/// - `params`: Structured values to pass as parameters
///
/// # Parameters
///
/// The `params` field can be omitted, a structured value (object), or an array.
/// The interpretation of params is up to the method implementation.
///
/// # Examples
///
/// ```rust
/// use jrow_core::{JsonRpcRequest, Id};
/// use serde_json::json;
///
/// // Request with object parameters
/// let req = JsonRpcRequest::new(
///     "subtract",
///     Some(json!({"minuend": 42, "subtrahend": 23})),
///     Id::Number(1)
/// );
///
/// // Request with no parameters
/// let req2 = JsonRpcRequest::new("getServerTime", None, Id::String("time-1".into()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version - always "2.0" for this specification
    pub jsonrpc: String,
    /// Name of the remote method to invoke
    pub method: String,
    /// Optional parameters to pass to the method
    /// Skipped in JSON if None to keep messages compact
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// Unique identifier to correlate this request with its response
    pub id: Id,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC 2.0 request
    ///
    /// The `jsonrpc` field is automatically set to "2.0" per the specification.
    ///
    /// # Arguments
    ///
    /// * `method` - The name of the method to invoke on the remote server
    /// * `params` - Optional parameters (use None if method takes no parameters)
    /// * `id` - Unique identifier for correlating the response
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jrow_core::{JsonRpcRequest, Id};
    ///
    /// let request = JsonRpcRequest::new("ping", None, Id::Number(1));
    /// assert_eq!(request.jsonrpc, "2.0");
    /// assert_eq!(request.method, "ping");
    /// ```
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>, id: Id) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
            id,
        }
    }
}

/// JSON-RPC 2.0 notification message
///
/// A notification is like a request, but crucially **does not expect a response**.
/// This is signaled by the absence of an `id` field. Notifications are useful for
/// fire-and-forget operations or server-to-client events in pub/sub scenarios.
///
/// # Why Notifications?
///
/// Notifications reduce overhead when you don't need confirmation:
/// - Broadcasting events to multiple clients
/// - Logging or telemetry where responses aren't needed
/// - One-way signals like "heartbeat" or "keepalive"
///
/// # Spec Compliance
///
/// Per JSON-RPC 2.0 spec, notifications MUST NOT include an `id` field.
/// The server MUST NOT send a response to a notification, even if an error occurs.
///
/// # Examples
///
/// ```rust
/// use jrow_core::JsonRpcNotification;
/// use serde_json::json;
///
/// // Notify about a status update
/// let notif = JsonRpcNotification::new(
///     "status.update",
///     Some(json!({"status": "online"}))
/// );
///
/// // No-parameter notification
/// let ping = JsonRpcNotification::new("ping", None);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// JSON-RPC version - always "2.0"
    pub jsonrpc: String,
    /// Name of the method/event being notified
    pub method: String,
    /// Optional parameters or event data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcNotification {
    /// Create a new JSON-RPC 2.0 notification
    ///
    /// Notifications do not have an ID and expect no response.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name or event type
    /// * `params` - Optional data associated with the notification
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jrow_core::JsonRpcNotification;
    /// use serde_json::json;
    ///
    /// let notif = JsonRpcNotification::new(
    ///     "user.joined",
    ///     Some(json!({"username": "alice"}))
    /// );
    /// ```
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response message
///
/// A response is sent by the server to the client after processing a request.
/// It contains either a result (success) or an error (failure), but never both.
///
/// # Spec Compliance
///
/// Per JSON-RPC 2.0 specification:
/// - `result`: Required on success, must not exist on error
/// - `error`: Required on error, must not exist on success
/// - `id`: Must match the `id` from the corresponding request
///
/// If there was an error detecting the request `id` (e.g. invalid JSON),
/// the response will use `Id::Null`.
///
/// # Success vs Error
///
/// The response MUST have exactly one of `result` or `error`:
/// - **Success**: `result` is present, `error` is None
/// - **Error**: `error` is present, `result` is None
///
/// This mutual exclusion is enforced by construction using the factory methods.
///
/// # Examples
///
/// ```rust
/// use jrow_core::{JsonRpcResponse, JsonRpcErrorData, Id};
/// use serde_json::json;
///
/// // Success response
/// let success = JsonRpcResponse::success(json!({"value": 42}), Id::Number(1));
/// assert!(success.is_success());
///
/// // Error response
/// let error = JsonRpcResponse::error(
///     JsonRpcErrorData::method_not_found("unknownMethod"),
///     Id::Number(2)
/// );
/// assert!(error.is_error());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version - always "2.0"
    pub jsonrpc: String,
    /// The result of the method invocation (present only on success)
    /// Mutually exclusive with `error`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error information (present only on failure)
    /// Mutually exclusive with `result`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorData>,
    /// Request ID from the original request (for correlation)
    /// Will be `Id::Null` if the request ID couldn't be determined
    pub id: Id,
}

impl JsonRpcResponse {
    /// Create a successful JSON-RPC 2.0 response
    ///
    /// The `error` field is automatically set to None.
    ///
    /// # Arguments
    ///
    /// * `result` - The successful result value to return
    /// * `id` - The request ID (must match the original request)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jrow_core::{JsonRpcResponse, Id};
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::success(json!({"status": "ok"}), Id::Number(1));
    /// assert!(response.is_success());
    /// ```
    pub fn success(result: serde_json::Value, id: Id) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error JSON-RPC 2.0 response
    ///
    /// The `result` field is automatically set to None.
    ///
    /// # Arguments
    ///
    /// * `error` - The error details
    /// * `id` - The request ID (use `Id::Null` if request ID couldn't be determined)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jrow_core::{JsonRpcResponse, JsonRpcErrorData, Id};
    ///
    /// let response = JsonRpcResponse::error(
    ///     JsonRpcErrorData::invalid_params("Missing required field"),
    ///     Id::Number(1)
    /// );
    /// assert!(response.is_error());
    /// ```
    pub fn error(error: JsonRpcErrorData, id: Id) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }

    /// Check if the response represents a successful result
    ///
    /// Returns true if `result` is present, false otherwise.
    pub fn is_success(&self) -> bool {
        self.result.is_some()
    }

    /// Check if the response represents an error
    ///
    /// Returns true if `error` is present, false otherwise.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

/// JSON-RPC error response (convenience type)
///
/// This is a specialized version of `JsonRpcResponse` that only represents errors.
/// It's a convenience type that makes the error-only nature explicit in function
/// signatures and type definitions.
///
/// # Why a Separate Type?
///
/// While `JsonRpcResponse` can represent both success and error, sometimes you want
/// to signal in your API that a function *only* returns errors (e.g., error handling
/// middleware). This type makes that intent clear.
///
/// # Conversion
///
/// This type can be easily converted to `JsonRpcResponse` using `.into()`.
///
/// # Examples
///
/// ```rust
/// use jrow_core::{JsonRpcError, JsonRpcResponse, JsonRpcErrorData, Id};
///
/// let error = JsonRpcError::new(
///     JsonRpcErrorData::method_not_found("test"),
///     Id::Number(1)
/// );
///
/// // Convert to response
/// let response: JsonRpcResponse = error.into();
/// assert!(response.is_error());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// JSON-RPC version - always "2.0"
    pub jsonrpc: String,
    /// The error details
    pub error: JsonRpcErrorData,
    /// Request ID (from the original request)
    pub id: Id,
}

impl JsonRpcError {
    /// Create a new JSON-RPC error message
    ///
    /// # Arguments
    ///
    /// * `error` - The error details
    /// * `id` - The request ID (use `Id::Null` if unknown)
    pub fn new(error: JsonRpcErrorData, id: Id) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            error,
            id,
        }
    }
}

impl From<JsonRpcError> for JsonRpcResponse {
    /// Convert a JsonRpcError into a JsonRpcResponse
    ///
    /// This is useful when you have an error but need to return a response type.
    fn from(err: JsonRpcError) -> Self {
        JsonRpcResponse::error(err.error, err.id)
    }
}

/// Unified enum representing any JSON-RPC 2.0 message
///
/// JSON-RPC messages can be requests, notifications, responses, or batches.
/// This enum provides a single type that can represent any of these variants,
/// which is useful for generic message handling.
///
/// # Batch Messages
///
/// The JSON-RPC 2.0 spec allows sending multiple messages in a single JSON array,
/// called a "batch". Batches can contain any mix of requests and notifications
/// (but not responses, which are returned as a separate batch).
///
/// The `Batch` variant stores raw `serde_json::Value` objects because individual
/// items need to be parsed separately (and may have parse errors independently).
///
/// # Untagged Serialization
///
/// This enum uses `#[serde(untagged)]` to serialize directly as the inner type
/// without adding a discriminator field. This allows the enum to match the
/// JSON-RPC 2.0 spec exactly.
///
/// # Why This Type?
///
/// When receiving messages over the wire, you don't know in advance whether
/// it's a request, notification, response, or batch. This enum allows you to
/// parse the message generically and then handle each variant appropriately.
///
/// # Examples
///
/// ```rust
/// use jrow_core::{JsonRpcMessage, codec};
///
/// // Parse an incoming message
/// let json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
/// let message = codec::decode(json).unwrap();
///
/// match message {
///     JsonRpcMessage::Request(req) => println!("Got request: {}", req.method),
///     JsonRpcMessage::Notification(notif) => println!("Got notification: {}", notif.method),
///     JsonRpcMessage::Response(resp) => println!("Got response"),
///     JsonRpcMessage::Batch(items) => println!("Got batch with {} items", items.len()),
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    /// A request message (expects a response)
    Request(JsonRpcRequest),
    /// A notification message (no response expected)
    Notification(JsonRpcNotification),
    /// A response message (result of processing a request)
    Response(JsonRpcResponse),
    /// A batch of messages (array of requests/notifications)
    /// Stored as raw values because each needs individual parsing
    Batch(Vec<serde_json::Value>),
}

impl JsonRpcMessage {
    /// Check if this message is a request
    ///
    /// Returns true for `Request` variant, false otherwise.
    pub fn is_request(&self) -> bool {
        matches!(self, JsonRpcMessage::Request(_))
    }

    /// Check if this message is a notification
    ///
    /// Returns true for `Notification` variant, false otherwise.
    pub fn is_notification(&self) -> bool {
        matches!(self, JsonRpcMessage::Notification(_))
    }

    /// Check if this message is a response
    ///
    /// Returns true for `Response` variant, false otherwise.
    pub fn is_response(&self) -> bool {
        matches!(self, JsonRpcMessage::Response(_))
    }

    /// Check if this message is a batch
    ///
    /// Returns true for `Batch` variant, false otherwise.
    pub fn is_batch(&self) -> bool {
        matches!(self, JsonRpcMessage::Batch(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_display() {
        assert_eq!(Id::String("test".to_string()).to_string(), "\"test\"");
        assert_eq!(Id::Number(42).to_string(), "42");
        assert_eq!(Id::Null.to_string(), "null");
    }

    #[test]
    fn test_request_serialization() {
        let req = JsonRpcRequest::new("test", None, Id::Number(1));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"test\""));
        assert!(json.contains("\"id\":1"));
    }

    #[test]
    fn test_notification_serialization() {
        let notif = JsonRpcNotification::new("notify", None);
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"notify\""));
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_response_success() {
        let resp = JsonRpcResponse::success(serde_json::json!({"status": "ok"}), Id::Number(1));
        assert!(resp.is_success());
        assert!(!resp.is_error());
    }

    #[test]
    fn test_response_error() {
        let resp = JsonRpcResponse::error(
            JsonRpcErrorData::internal_error("test error"),
            Id::Number(1),
        );
        assert!(!resp.is_success());
        assert!(resp.is_error());
    }
}
