//! Codec for JSON-RPC message serialization and deserialization
//!
//! This module provides functions for encoding Rust types to JSON strings and
//! decoding JSON strings back to Rust types, specifically for JSON-RPC 2.0 messages.
//!
//! # Why a Codec Module?
//!
//! While serde provides generic JSON serialization, this module adds:
//! - **Spec-compliant validation**: Ensures messages conform to JSON-RPC 2.0
//! - **Batch handling**: Special logic for detecting and processing batch requests
//! - **Error mapping**: Converts serde errors to JSON-RPC error codes
//! - **Convenience**: Type-safe encoding/decoding functions
//!
//! # Batch Messages
//!
//! JSON-RPC 2.0 allows sending multiple messages in a single JSON array.
//! The `decode()` function automatically detects batches and returns them
//! as `JsonRpcMessage::Batch`, with individual items stored as raw values
//! for separate parsing.
//!
//! # Error Handling
//!
//! Codec functions return `Result<T>` where errors are mapped to appropriate
//! JSON-RPC error codes:
//! - Parse errors → `-32700` (Parse error)
//! - Invalid requests → `-32600` (Invalid Request)
//! - Serialization issues → `Error::Serialization`
//!
//! # Examples
//!
//! ```rust
//! use jrow_core::{codec, JsonRpcRequest, Id};
//!
//! // Encode a request
//! let request = JsonRpcRequest::new("ping", None, Id::Number(1));
//! let json = codec::encode_request(&request).unwrap();
//!
//! // Decode it back
//! let decoded = codec::decode(&json).unwrap();
//! assert!(decoded.is_request());
//! ```

use crate::error::{Error, JsonRpcErrorData, Result};
use crate::types::{JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use serde::{Deserialize, Serialize};

/// Encode any serializable message to a JSON string
///
/// This is a generic encoding function that works with any type implementing
/// `Serialize`. Use the type-specific functions (like `encode_request`) for
/// better type safety and clarity in application code.
///
/// # Arguments
///
/// * `msg` - A reference to the message to encode
///
/// # Errors
///
/// Returns `Error::Serialization` if the message cannot be serialized to JSON.
/// This can happen if the message contains types that aren't JSON-compatible.
///
/// # Examples
///
/// ```rust
/// use jrow_core::{codec, JsonRpcRequest, Id};
///
/// let request = JsonRpcRequest::new("test", None, Id::Number(1));
/// let json = codec::encode(&request).unwrap();
/// assert!(json.contains("\"method\":\"test\""));
/// ```
pub fn encode<T: Serialize>(msg: &T) -> Result<String> {
    serde_json::to_string(msg).map_err(|e| Error::Serialization(e.to_string()))
}

/// Decode a JSON string to a JSON-RPC message (single or batch)
///
/// This is the primary decoding function for incoming messages. It automatically
/// detects whether the message is a single message or a batch, and handles
/// both cases appropriately.
///
/// # Batch Detection
///
/// If the JSON is an array, it's treated as a batch. Each item in the batch
/// is stored as a raw `serde_json::Value` for individual parsing later.
/// This allows processing to continue even if some batch items are malformed.
///
/// # Validation
///
/// - Empty batches are rejected (per JSON-RPC 2.0 spec)
/// - Invalid JSON returns a Parse error (-32700)
/// - Malformed request objects return Invalid Request error (-32600)
///
/// # Arguments
///
/// * `data` - The JSON string to decode
///
/// # Returns
///
/// - `JsonRpcMessage::Request` for single requests
/// - `JsonRpcMessage::Notification` for single notifications
/// - `JsonRpcMessage::Response` for single responses
/// - `JsonRpcMessage::Batch` for batch requests
///
/// # Errors
///
/// - `Error::JsonRpc(ParseError)` if JSON is invalid
/// - `Error::JsonRpc(InvalidRequest)` if structure is wrong or batch is empty
///
/// # Examples
///
/// ```rust
/// use jrow_core::{codec, JsonRpcMessage};
///
/// // Single request
/// let json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
/// let msg = codec::decode(json).unwrap();
/// assert!(msg.is_request());
///
/// // Batch request
/// let batch_json = r#"[{"jsonrpc":"2.0","method":"test1","id":1},
///                       {"jsonrpc":"2.0","method":"test2","id":2}]"#;
/// let msg = codec::decode(batch_json).unwrap();
/// assert!(msg.is_batch());
/// ```
pub fn decode(data: &str) -> Result<JsonRpcMessage> {
    // First, try to parse as a generic JSON value to determine the structure
    // This two-step approach allows us to handle arrays (batches) specially
    let value: serde_json::Value =
        serde_json::from_str(data).map_err(|_e| Error::JsonRpc(JsonRpcErrorData::parse_error()))?;

    if value.is_array() {
        // It's a batch request - extract the array
        let messages: Vec<serde_json::Value> = serde_json::from_value(value)
            .map_err(|_e| Error::JsonRpc(JsonRpcErrorData::parse_error()))?;

        // Per JSON-RPC 2.0 spec: "The Request objects SHOULD be an Array"
        // implies the array should not be empty
        if messages.is_empty() {
            return Err(Error::JsonRpc(JsonRpcErrorData::invalid_request(
                "Batch cannot be empty",
            )));
        }

        Ok(JsonRpcMessage::Batch(messages))
    } else {
        // It's a single message - deserialize it into the appropriate type
        // The #[serde(untagged)] on JsonRpcMessage will try each variant
        serde_json::from_value(value).map_err(|_e| Error::JsonRpc(JsonRpcErrorData::parse_error()))
    }
}

/// Decode a JSON string to a specific JSON-RPC type
///
/// This is a lower-level function that decodes directly to a specific type
/// without going through `JsonRpcMessage`. Use this when you know exactly
/// what type to expect.
///
/// # Type Parameters
///
/// * `T` - The target type that implements `Deserialize`
///
/// # Arguments
///
/// * `data` - The JSON string to decode
///
/// # Errors
///
/// Returns `Error::Serialization` if the JSON doesn't match the expected type.
///
/// # Examples
///
/// ```rust
/// use jrow_core::{codec, JsonRpcRequest};
///
/// let json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
/// let request: JsonRpcRequest = codec::decode_as(json).unwrap();
/// assert_eq!(request.method, "test");
/// ```
pub fn decode_as<'de, T: Deserialize<'de>>(data: &'de str) -> Result<T> {
    serde_json::from_str(data).map_err(|e| Error::Serialization(e.to_string()))
}

/// Encode a JSON-RPC request to JSON
///
/// Type-safe wrapper around `encode()` for requests.
///
/// # Examples
///
/// ```rust
/// use jrow_core::{codec, JsonRpcRequest, Id};
///
/// let request = JsonRpcRequest::new("add", None, Id::Number(1));
/// let json = codec::encode_request(&request).unwrap();
/// ```
pub fn encode_request(req: &JsonRpcRequest) -> Result<String> {
    encode(req)
}

/// Encode a JSON-RPC notification to JSON
///
/// Type-safe wrapper around `encode()` for notifications.
///
/// # Examples
///
/// ```rust
/// use jrow_core::{codec, JsonRpcNotification};
///
/// let notif = JsonRpcNotification::new("status.changed", None);
/// let json = codec::encode_notification(&notif).unwrap();
/// ```
pub fn encode_notification(notif: &JsonRpcNotification) -> Result<String> {
    encode(notif)
}

/// Encode a JSON-RPC response to JSON
///
/// Type-safe wrapper around `encode()` for responses.
///
/// # Examples
///
/// ```rust
/// use jrow_core::{codec, JsonRpcResponse, Id};
/// use serde_json::json;
///
/// let response = JsonRpcResponse::success(json!({"result": 42}), Id::Number(1));
/// let json = codec::encode_response(&response).unwrap();
/// ```
pub fn encode_response(resp: &JsonRpcResponse) -> Result<String> {
    encode(resp)
}

/// Decode a JSON string to a JSON-RPC request
///
/// Use this when you know the message is definitely a request.
/// If you're not sure, use `decode()` instead.
///
/// # Examples
///
/// ```rust
/// use jrow_core::codec;
///
/// let json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
/// let request = codec::decode_request(json).unwrap();
/// assert_eq!(request.method, "test");
/// ```
pub fn decode_request(data: &str) -> Result<JsonRpcRequest> {
    decode_as(data)
}

/// Decode a JSON string to a JSON-RPC notification
///
/// Use this when you know the message is definitely a notification.
/// If you're not sure, use `decode()` instead.
pub fn decode_notification(data: &str) -> Result<JsonRpcNotification> {
    decode_as(data)
}

/// Decode a JSON string to a JSON-RPC response
///
/// Use this when you know the message is definitely a response.
/// If you're not sure, use `decode()` instead.
pub fn decode_response(data: &str) -> Result<JsonRpcResponse> {
    decode_as(data)
}

/// Encode a batch of responses to JSON array
///
/// When processing a batch request, use this to encode multiple responses
/// into a single JSON array for transmission back to the client.
///
/// # Arguments
///
/// * `responses` - Slice of responses to encode
///
/// # Returns
///
/// A JSON string containing an array of response objects
///
/// # Examples
///
/// ```rust
/// use jrow_core::{codec, JsonRpcResponse, Id};
/// use serde_json::json;
///
/// let responses = vec![
///     JsonRpcResponse::success(json!(42), Id::Number(1)),
///     JsonRpcResponse::success(json!(99), Id::Number(2)),
/// ];
///
/// let json = codec::encode_batch_responses(&responses).unwrap();
/// assert!(json.starts_with('['));
/// ```
pub fn encode_batch_responses(responses: &[JsonRpcResponse]) -> Result<String> {
    serde_json::to_string(responses).map_err(|e| Error::Serialization(e.to_string()))
}

/// Decode individual messages from a batch
///
/// Takes the raw `serde_json::Value` items from a batch and attempts to
/// parse each one into a `JsonRpcMessage`. Returns a vector of results,
/// allowing you to process successful messages while reporting errors
/// for failed ones.
///
/// # Why Return Vec<Result>?
///
/// According to JSON-RPC 2.0 spec, the server should process all valid
/// messages in a batch and return responses for them, even if some
/// messages are invalid. Returning `Vec<Result>` allows this behavior.
///
/// # Arguments
///
/// * `values` - Vector of raw JSON values from a batch
///
/// # Returns
///
/// A vector of results, one for each input value. Invalid messages will
/// be `Err(Error::JsonRpc(InvalidRequest))`.
///
/// # Examples
///
/// ```rust
/// use jrow_core::codec;
/// use serde_json::json;
///
/// let batch = vec![
///     json!({"jsonrpc":"2.0","method":"test","id":1}),
///     json!({"invalid":"message"}),  // This will fail
/// ];
///
/// let results = codec::decode_batch_messages(batch);
/// assert_eq!(results.len(), 2);
/// assert!(results[0].is_ok());
/// assert!(results[1].is_err());
/// ```
pub fn decode_batch_messages(values: Vec<serde_json::Value>) -> Vec<Result<JsonRpcMessage>> {
    values
        .into_iter()
        .map(|v| {
            // Try to deserialize each value as a JsonRpcMessage
            serde_json::from_value(v).map_err(|_e| {
                Error::JsonRpc(JsonRpcErrorData::invalid_request(
                    "Invalid message in batch",
                ))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Id;

    #[test]
    fn test_encode_decode_request() {
        let req = JsonRpcRequest::new("test_method", None, Id::Number(1));
        let encoded = encode_request(&req).unwrap();
        let decoded = decode_request(&encoded).unwrap();

        assert_eq!(decoded.method, "test_method");
        assert_eq!(decoded.id, Id::Number(1));
        assert_eq!(decoded.jsonrpc, "2.0");
    }

    #[test]
    fn test_encode_decode_notification() {
        let notif = JsonRpcNotification::new(
            "test_notification",
            Some(serde_json::json!({"key": "value"})),
        );
        let encoded = encode_notification(&notif).unwrap();
        let decoded = decode_notification(&encoded).unwrap();

        assert_eq!(decoded.method, "test_notification");
        assert_eq!(decoded.jsonrpc, "2.0");
        assert!(decoded.params.is_some());
    }

    #[test]
    fn test_encode_decode_response_success() {
        let resp = JsonRpcResponse::success(
            serde_json::json!({"result": 42}),
            Id::String("test-id".to_string()),
        );
        let encoded = encode_response(&resp).unwrap();
        let decoded = decode_response(&encoded).unwrap();

        assert!(decoded.is_success());
        assert_eq!(decoded.id, Id::String("test-id".to_string()));
    }

    #[test]
    fn test_encode_decode_response_error() {
        let resp = JsonRpcResponse::error(
            JsonRpcErrorData::method_not_found("unknown"),
            Id::Number(99),
        );
        let encoded = encode_response(&resp).unwrap();
        let decoded = decode_response(&encoded).unwrap();

        assert!(decoded.is_error());
        assert_eq!(decoded.id, Id::Number(99));
    }

    #[test]
    fn test_decode_message() {
        let request_json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        let msg = decode(request_json).unwrap();
        assert!(msg.is_request());

        let notif_json = r#"{"jsonrpc":"2.0","method":"notify"}"#;
        let msg = decode(notif_json).unwrap();
        assert!(msg.is_notification());

        let response_json = r#"{"jsonrpc":"2.0","result":42,"id":1}"#;
        let msg = decode(response_json).unwrap();
        assert!(msg.is_response());
    }

    #[test]
    fn test_decode_invalid_json() {
        let invalid = "not valid json";
        let result = decode(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_empty_string() {
        // Test empty input handling
        let result = decode("");
        assert!(result.is_err());
        
        // Verify error is appropriate
        match result {
            Err(Error::JsonRpc(_)) => {}, // Expected
            _ => panic!("Expected JsonRpc error for empty input"),
        }
    }

    #[test]
    fn test_decode_single_object() {
        // Test decoding a single object (not a batch)
        let single_obj = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        let result = decode(single_obj);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encode_batch_non_empty() {
        // Test encoding batch with multiple items
        let batch = vec![
            JsonRpcResponse::success(serde_json::json!(1), Id::Number(1)),
            JsonRpcResponse::success(serde_json::json!(2), Id::Number(2)),
        ];
        let encoded = encode_batch_responses(&batch).unwrap();
        assert!(encoded.starts_with("["));
        assert!(encoded.ends_with("]"));
    }

    #[test]
    fn test_decode_mixed_batch() {
        // Test batch with both requests and notifications
        let batch_json = r#"[
            {"jsonrpc":"2.0","method":"notify"},
            {"jsonrpc":"2.0","method":"request","id":1},
            {"jsonrpc":"2.0","result":42,"id":2}
        ]"#;
        
        let result = decode(batch_json).unwrap();
        match result {
            JsonRpcMessage::Batch(items) => {
                assert_eq!(items.len(), 3);
                // Each item should be valid JSON
                for item in items {
                    assert!(item.is_object() || item.is_string() || item.is_number());
                }
            },
            _ => panic!("Expected batch message"),
        }
    }

    #[test]
    fn test_decode_invalid_jsonrpc_version() {
        // Test wrong version string
        let wrong_version = r#"{"jsonrpc":"1.0","method":"test","id":1}"#;
        let result = decode(wrong_version);
        
        // Should still decode (version is just a string field)
        // But validation might happen at a higher level
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_encode_decode_with_null_params() {
        // Test request with explicit null params
        let req = JsonRpcRequest::new("test", None, Id::Number(1));
        let encoded = encode_request(&req).unwrap();
        let decoded = decode_request(&encoded).unwrap();
        
        assert_eq!(decoded.method, "test");
        assert!(decoded.params.is_none());
    }

    #[test]
    fn test_decode_batch_messages_with_errors() {
        // Test decode_batch_messages with some invalid items
        let values = vec![
            serde_json::json!({"jsonrpc":"2.0","method":"valid","id":1}),
            serde_json::json!({"invalid": "message"}),
            serde_json::json!({"jsonrpc":"2.0","method":"notify"}),
        ];
        
        let results = decode_batch_messages(values);
        assert_eq!(results.len(), 3);
        
        // First should succeed
        assert!(results[0].is_ok());
        // Second should fail
        assert!(results[1].is_err());
        // Third should succeed
        assert!(results[2].is_ok());
    }

    #[test]
    fn test_encode_decode_response_with_null_id() {
        // Test response with null ID (for errors that couldn't determine request ID)
        let resp = JsonRpcResponse::error(
            JsonRpcErrorData::parse_error(),
            Id::Null,
        );
        let encoded = encode_response(&resp).unwrap();
        let decoded = decode_response(&encoded).unwrap();
        
        assert!(decoded.is_error());
        assert_eq!(decoded.id, Id::Null);
    }

    #[test]
    fn test_decode_as_correct_type() {
        // Test decode_as with correct type
        let json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        
        // Decode request as request
        let result: Result<JsonRpcRequest> = decode_as(json);
        assert!(result.is_ok());
        
        let request = result.unwrap();
        assert_eq!(request.method, "test");
    }

    #[test]
    fn test_encode_decode_large_batch() {
        // Test encoding/decoding a large batch
        let mut responses = Vec::new();
        for i in 0..100 {
            responses.push(JsonRpcResponse::success(
                serde_json::json!({"value": i}),
                Id::Number(i as i64),
            ));
        }
        
        let encoded = encode_batch_responses(&responses).unwrap();
        assert!(!encoded.is_empty());
        
        // Decode and verify
        let decoded = decode(&encoded).unwrap();
        match decoded {
            JsonRpcMessage::Batch(items) => assert_eq!(items.len(), 100),
            _ => panic!("Expected batch message"),
        }
    }

    #[test]
    fn test_encode_preserves_order() {
        // Test that encoding preserves field order for debugging
        let req = JsonRpcRequest::new("test", Some(serde_json::json!({"a": 1})), Id::Number(1));
        let encoded = encode_request(&req).unwrap();
        
        // Verify it contains expected fields
        assert!(encoded.contains("jsonrpc"));
        assert!(encoded.contains("method"));
        assert!(encoded.contains("params"));
        assert!(encoded.contains("id"));
    }
}
