//! Error types for jrow
//!
//! This module provides comprehensive error handling for JSON-RPC 2.0 operations.
//! It defines two main error types:
//!
//! - **Error**: Application-level errors for internal use (uses thiserror)
//! - **JsonRpcErrorData**: Wire-format errors as defined in the JSON-RPC 2.0 spec
//!
//! # Error Hierarchy
//!
//! The `Error` enum covers all error conditions that can occur during JSON-RPC
//! operations, from network issues to protocol violations. It can be converted
//! into `JsonRpcErrorData` for transmission over the wire.
//!
//! # Spec-Compliant Error Codes
//!
//! JSON-RPC 2.0 defines standard error codes:
//! - `-32700`: Parse error (invalid JSON)
//! - `-32600`: Invalid request (missing required fields)
//! - `-32601`: Method not found
//! - `-32602`: Invalid params
//! - `-32603`: Internal error
//! - `-32000 to -32099`: Server error (implementation-defined)
//!
//! # Examples
//!
//! ```rust
//! use jrow_core::{Error, JsonRpcErrorData};
//!
//! // Application error
//! let error = Error::MethodNotFound("unknownMethod".into());
//!
//! // Create corresponding JSON-RPC error data
//! let json_error = JsonRpcErrorData::method_not_found("unknownMethod");
//! assert_eq!(json_error.code, -32601);
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result type for jrow operations
///
/// This is a convenience type alias that uses the jrow `Error` type.
/// Used throughout the jrow crates for consistent error handling.
pub type Result<T> = std::result::Result<T, Error>;

/// Application-level error type for jrow operations
///
/// This enum represents all possible error conditions that can occur during
/// JSON-RPC operations. It provides rich context for debugging while being
/// convertible to wire-format `JsonRpcErrorData` for transmission.
///
/// # Error Categories
///
/// - **Protocol errors**: InvalidRequest, MethodNotFound, InvalidParams
/// - **Transport errors**: WebSocket, Io, ConnectionClosed
/// - **Processing errors**: Serialization, Internal
/// - **Operational errors**: Timeout, BatchSizeExceeded
///
/// # Usage with thiserror
///
/// This enum uses the `thiserror` crate to automatically implement
/// `std::error::Error` and provide nice error messages.
///
/// # Conversion to JSON-RPC Errors
///
/// These errors can be converted into `JsonRpcErrorData` for sending
/// over the wire, mapping to standard JSON-RPC 2.0 error codes.
#[derive(Debug, Clone, Error)]
pub enum Error {
    /// JSON-RPC protocol error (already in wire format)
    ///
    /// This variant holds errors that are already structured as JSON-RPC
    /// error objects, typically received from a remote peer.
    #[error("JSON-RPC error: {0}")]
    JsonRpc(#[from] JsonRpcErrorData),

    /// Serialization or deserialization error
    ///
    /// Occurs when converting between Rust types and JSON. Usually indicates
    /// a mismatch between expected and actual data structures.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// WebSocket transport layer error
    ///
    /// Covers connection issues, protocol violations, or frame processing errors
    /// at the WebSocket level (below JSON-RPC).
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Input/output error
    ///
    /// Low-level I/O errors from the operating system, such as network
    /// failures or file system issues.
    #[error("IO error: {0}")]
    Io(String),

    /// Invalid JSON-RPC request format
    ///
    /// The request is not well-formed according to JSON-RPC 2.0 spec
    /// (e.g., missing required fields, wrong types).
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Method not found on the server
    ///
    /// The requested method name doesn't exist in the server's handler registry.
    /// Maps to JSON-RPC error code -32601.
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// Invalid method parameters
    ///
    /// The method exists but the parameters are incorrect (wrong type, missing
    /// required params, etc.). Maps to JSON-RPC error code -32602.
    #[error("Invalid params: {0}")]
    InvalidParams(String),

    /// Internal server error
    ///
    /// An unexpected error occurred during method execution. Maps to
    /// JSON-RPC error code -32603. Should be used sparingly; prefer
    /// more specific error types when possible.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Request operation timeout
    ///
    /// The operation took too long and was cancelled. Typically applies
    /// to client-side request timeouts, not part of JSON-RPC spec.
    #[error("Request timeout")]
    Timeout,

    /// Connection was closed
    ///
    /// The WebSocket connection is no longer active. Further operations
    /// will fail until reconnection occurs.
    #[error("Connection closed")]
    ConnectionClosed,

    /// Batch request size exceeded limit
    ///
    /// The batch contains more requests than the server's configured limit.
    /// This protects against denial-of-service via extremely large batches.
    #[error("Batch size limit exceeded: limit={limit}, actual={actual}")]
    BatchSizeExceeded {
        /// The maximum allowed batch size
        limit: usize,
        /// The actual batch size that was rejected
        actual: usize
    },
}

/// JSON-RPC 2.0 error data as defined in the specification
///
/// This structure represents the exact wire format for JSON-RPC errors.
/// It appears in the `error` field of a `JsonRpcResponse`.
///
/// # Spec Compliance
///
/// According to JSON-RPC 2.0 spec, error objects MUST contain:
/// - `code`: An integer error code
/// - `message`: A short description of the error
///
/// And MAY contain:
/// - `data`: Additional information about the error
///
/// # Standard Error Codes
///
/// The spec defines these reserved error codes:
/// - `-32700`: Parse error
/// - `-32600`: Invalid Request
/// - `-32601`: Method not found
/// - `-32602`: Invalid params
/// - `-32603`: Internal error
/// - `-32000 to -32099`: Server error (reserved for implementation-defined errors)
///
/// # Custom Error Codes
///
/// Applications can define custom error codes outside the reserved ranges.
/// By convention, positive error codes are application-specific.
///
/// # Examples
///
/// ```rust
/// use jrow_core::JsonRpcErrorData;
/// use serde_json::json;
///
/// // Standard error
/// let error = JsonRpcErrorData::method_not_found("calculate");
/// assert_eq!(error.code, -32601);
///
/// // Custom error with additional data
/// let custom = JsonRpcErrorData::with_data(
///     1001,
///     "Insufficient funds",
///     json!({"balance": 50, "required": 100})
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorData {
    /// Numeric error code indicating the error type
    ///
    /// Negative codes from -32768 to -32000 are reserved by the spec.
    pub code: i32,
    
    /// Human-readable error message
    ///
    /// Should be a short sentence describing the error. For example:
    /// "Method not found", "Invalid params", etc.
    pub message: String,
    
    /// Optional additional error information
    ///
    /// Can contain any JSON-serializable data that provides more context
    /// about the error. For example: stack traces, validation errors, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcErrorData {
    /// Create a new JSON-RPC error with code and message
    ///
    /// Use the standard error factory methods (like `parse_error()`) for
    /// spec-defined errors, or this constructor for custom application errors.
    ///
    /// # Arguments
    ///
    /// * `code` - Numeric error code (use -32000 to -32099 for server errors)
    /// * `message` - Human-readable error description
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jrow_core::JsonRpcErrorData;
    ///
    /// let error = JsonRpcErrorData::new(-32000, "Database connection failed");
    /// ```
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create a new JSON-RPC error with additional data
    ///
    /// The `data` field can contain any contextual information about the error,
    /// such as validation details, stack traces, or debugging information.
    ///
    /// # Arguments
    ///
    /// * `code` - Numeric error code
    /// * `message` - Human-readable error description
    /// * `data` - Additional structured error information
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jrow_core::JsonRpcErrorData;
    /// use serde_json::json;
    ///
    /// let error = JsonRpcErrorData::with_data(
    ///     -32602,
    ///     "Invalid params",
    ///     json!({"missing": ["username", "password"]})
    /// );
    /// ```
    pub fn with_data(code: i32, message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Create a parse error (-32700)
    ///
    /// Indicates that the server received invalid JSON. This cannot be
    /// determined until attempting to parse the request.
    ///
    /// Per spec: "Invalid JSON was received by the server. An error occurred
    /// on the server while parsing the JSON text."
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    /// Create an invalid request error (-32600)
    ///
    /// Indicates that the JSON is valid, but the request object is malformed
    /// (missing required fields, wrong types, etc.).
    ///
    /// Per spec: "The JSON sent is not a valid Request object."
    ///
    /// # Arguments
    ///
    /// * `msg` - Specific reason why the request is invalid
    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self::new(-32600, msg)
    }

    /// Create a method not found error (-32601)
    ///
    /// Indicates that the requested method doesn't exist on the server.
    ///
    /// Per spec: "The method does not exist / is not available."
    ///
    /// # Arguments
    ///
    /// * `method` - The name of the method that wasn't found
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jrow_core::JsonRpcErrorData;
    ///
    /// let error = JsonRpcErrorData::method_not_found("calculateFoo");
    /// assert_eq!(error.message, "Method not found: calculateFoo");
    /// ```
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::new(-32601, format!("Method not found: {}", method.into()))
    }

    /// Create an invalid params error (-32602)
    ///
    /// Indicates that the method was found, but the parameters are invalid
    /// (wrong type, missing required params, etc.).
    ///
    /// Per spec: "Invalid method parameter(s)."
    ///
    /// # Arguments
    ///
    /// * `msg` - Specific reason why the params are invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jrow_core::JsonRpcErrorData;
    ///
    /// let error = JsonRpcErrorData::invalid_params("Expected numeric 'amount' parameter");
    /// ```
    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self::new(-32602, msg)
    }

    /// Create an internal error (-32603)
    ///
    /// Indicates an unexpected error occurred on the server during method execution.
    ///
    /// Per spec: "Internal JSON-RPC error."
    ///
    /// # When to Use
    ///
    /// Use this for unexpected errors that don't fit other categories. For
    /// application-specific errors, consider using custom error codes instead
    /// (e.g., -32000 to -32099 range).
    ///
    /// # Arguments
    ///
    /// * `msg` - Description of the internal error
    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::new(-32603, msg)
    }

    /// Create a batch size exceeded error (-32600)
    ///
    /// Indicates that the batch request contains too many items.
    /// This is a jrow-specific error that helps prevent denial-of-service
    /// attacks via extremely large batch requests.
    ///
    /// # Arguments
    ///
    /// * `limit` - The configured maximum batch size
    /// * `actual` - The actual size of the rejected batch
    pub fn batch_size_exceeded(limit: usize, actual: usize) -> Self {
        Self::new(
            -32600,
            format!("Batch size limit exceeded: limit={}, actual={}", limit, actual),
        )
    }
}

impl std::fmt::Display for JsonRpcErrorData {
    /// Format the error for display
    ///
    /// Formats as "[code] message" for easy readability in logs.
    /// For example: "[-32601] Method not found: unknownMethod"
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

// Implement std::error::Error so JsonRpcErrorData can be used with Result and ?
impl std::error::Error for JsonRpcErrorData {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_error_from_serde() {
        // Test conversion from serde_json::Error
        let json_str = r#"{"invalid": json"#;
        let serde_error = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let error = Error::Serialization(serde_error.to_string());
        
        match error {
            Error::Serialization(msg) => assert!(!msg.is_empty()),
            _ => panic!("Expected Serialization error"),
        }
    }

    #[test]
    fn test_error_from_io() {
        // Test IO error conversion
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error = Error::Io(io_error.to_string());
        
        match error {
            Error::Io(msg) => assert_eq!(msg, "file not found"),
            _ => panic!("Expected IO error"),
        }
    }

    #[test]
    fn test_jsonrpc_error_with_data() {
        let error = JsonRpcErrorData::with_data(
            -32602,
            "Invalid params",
            json!({"missing": ["username", "password"]}),
        );
        
        assert_eq!(error.code, -32602);
        assert_eq!(error.message, "Invalid params");
        assert!(error.data.is_some());
        
        if let Some(data) = error.data {
            assert_eq!(data["missing"][0], "username");
            assert_eq!(data["missing"][1], "password");
        }
    }

    #[test]
    fn test_jsonrpc_error_display() {
        let error = JsonRpcErrorData::method_not_found("unknownMethod");
        let display = format!("{}", error);
        
        assert!(display.contains("-32601"));
        assert!(display.contains("Method not found"));
    }

    #[test]
    fn test_parse_error_creation() {
        let error = JsonRpcErrorData::parse_error();
        
        assert_eq!(error.code, -32700);
        assert_eq!(error.message, "Parse error");
        assert!(error.data.is_none());
    }

    #[test]
    fn test_invalid_request_creation() {
        let error = JsonRpcErrorData::invalid_request("Missing 'id' field");
        
        assert_eq!(error.code, -32600);
        assert!(error.message.contains("Missing 'id' field"));
    }

    #[test]
    fn test_method_not_found_creation() {
        let error = JsonRpcErrorData::method_not_found("testMethod");
        
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("testMethod"));
    }

    #[test]
    fn test_invalid_params_creation() {
        let error = JsonRpcErrorData::invalid_params("Expected number, got string");
        
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("Expected number"));
    }

    #[test]
    fn test_internal_error_creation() {
        let error = JsonRpcErrorData::internal_error("Database connection failed");
        
        assert_eq!(error.code, -32603);
        assert!(error.message.contains("Database connection failed"));
    }

    #[test]
    fn test_batch_size_exceeded_creation() {
        let error = JsonRpcErrorData::batch_size_exceeded(100, 150);
        
        assert_eq!(error.code, -32600);
        assert!(error.message.contains("100"));
        assert!(error.message.contains("150"));
    }

    #[test]
    fn test_all_jsonrpc_error_codes() {
        // Verify all standard JSON-RPC 2.0 error codes
        let errors = vec![
            (JsonRpcErrorData::parse_error(), -32700),
            (JsonRpcErrorData::invalid_request("test"), -32600),
            (JsonRpcErrorData::method_not_found("test"), -32601),
            (JsonRpcErrorData::invalid_params("test"), -32602),
            (JsonRpcErrorData::internal_error("test"), -32603),
        ];
        
        for (error, expected_code) in errors {
            assert_eq!(error.code, expected_code);
            assert!(!error.message.is_empty());
        }
    }

    #[test]
    fn test_error_serialization() {
        // Test that JsonRpcErrorData can be serialized
        let error = JsonRpcErrorData::new(-32000, "Custom error");
        let serialized = serde_json::to_string(&error).unwrap();
        
        assert!(serialized.contains("-32000"));
        assert!(serialized.contains("Custom error"));
    }

    #[test]
    fn test_error_deserialization() {
        // Test that JsonRpcErrorData can be deserialized
        let json = r#"{"code":-32601,"message":"Method not found"}"#;
        let error: JsonRpcErrorData = serde_json::from_str(json).unwrap();
        
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
        assert!(error.data.is_none());
    }

    #[test]
    fn test_error_with_data_serialization() {
        let error = JsonRpcErrorData::with_data(
            -32000,
            "Test error",
            json!({"key": "value"}),
        );
        
        let serialized = serde_json::to_string(&error).unwrap();
        let deserialized: JsonRpcErrorData = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.code, error.code);
        assert_eq!(deserialized.message, error.message);
        assert_eq!(deserialized.data, error.data);
    }

    #[test]
    fn test_error_display_formatting() {
        let error = Error::MethodNotFound("testMethod".to_string());
        let display = format!("{}", error);
        
        assert!(display.contains("testMethod"));
    }

    #[test]
    fn test_connection_closed_error() {
        let error = Error::ConnectionClosed;
        match error {
            Error::ConnectionClosed => {}, // Expected
            _ => panic!("Expected ConnectionClosed error"),
        }
    }

    #[test]
    fn test_timeout_error() {
        let error = Error::Timeout;
        match error {
            Error::Timeout => {}, // Expected
            _ => panic!("Expected Timeout error"),
        }
    }

    #[test]
    fn test_invalid_request_error() {
        let error = Error::InvalidRequest("Missing id field".to_string());
        match error {
            Error::InvalidRequest(msg) => assert_eq!(msg, "Missing id field"),
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_websocket_error() {
        let error = Error::WebSocket("Protocol error".to_string());
        match error {
            Error::WebSocket(msg) => assert_eq!(msg, "Protocol error"),
            _ => panic!("Expected WebSocket error"),
        }
    }
}
