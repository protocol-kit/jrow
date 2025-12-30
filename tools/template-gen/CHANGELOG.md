# Template Generator Changelog

## Schema-First Update (2025-12-27)

### Overview

Updated the JROW template generator to support the new Schema-First AsyncAPI template with full type safety features.

### New Features

#### 1. Enhanced Configuration Structures

**ProjectConfig:**
- Added `contact_email` field for contact information

**ServerConfig:**
- Added `max_request_size: Option<u64>` - Maximum request size in bytes
- Added `max_batch_size: Option<u32>` - Maximum number of requests per batch

**AsyncApiConfig:**
- Added `rate_limit_enabled: Option<bool>` - Enable rate limiting
- Added `rate_limit_requests: Option<u32>` - Requests per window
- Added `rate_limit_window: Option<String>` - Time window (e.g., "60s")
- Added `error_codes: Vec<ErrorCode>` - Error code catalog

#### 2. New ErrorCode Structure

Complete error code definitions with:
```rust
struct ErrorCode {
    code: i32,              // Numeric error code
    name: String,           // Error name (e.g., "InvalidParams")
    message: String,        // Error message
    description: String,    // Detailed description
}
```

#### 3. Enhanced RpcMethod Structure

**New fields for full type safety:**
- `description: Option<String>` - Method description
- `tags: Option<Vec<String>>` - Tags for organization
- `deprecated: Option<bool>` - Deprecation flag
- `params_type: Option<String>` - Parameter type (object/array)
- `params_required: Option<Vec<String>>` - Required parameter names
- `params_properties: Option<String>` - JSON Schema for parameters
- `result_type: Option<String>` - Result type
- `result_description: Option<String>` - Result description
- `result_schema: Option<String>` - JSON Schema for result
- `result_examples: Option<Vec<String>>` - Multiple result examples
- `error_codes: Option<Vec<i32>>` - Associated error codes

**Kept for backwards compatibility:**
- `example_params: Option<String>` - Simple parameter example
- `example_result: Option<String>` - Simple result example

#### 4. Enhanced PubSubTopic Structure

**New fields for message schemas:**
- `description: Option<String>` - Topic description
- `tags: Option<Vec<String>>` - Tags for organization
- `pattern_type: Option<String>` - "exact" or "wildcard"
- `message_type: Option<String>` - Message type (object/array)
- `message_required: Option<Vec<String>>` - Required message fields
- `message_properties: Option<String>` - JSON Schema for message
- `publish_rate: Option<String>` - Expected publish frequency

**Kept for backwards compatibility:**
- `example_params: Option<String>` - Simple message example

### Default Configuration Updates

#### Error Codes

Added 5 standard JSON-RPC error codes:
- `-32700` ParseError
- `-32600` InvalidRequest
- `-32601` MethodNotFound
- `-32602` InvalidParams
- `-32603` InternalError

#### Example Methods

**1. Enhanced "add" method:**
```toml
name = "add"
description = "Add two numbers together"
tags = ["math", "calculation"]
params_type = "object"
params_properties = JSON Schema with validation
result_type = "number"
error_codes = [-32602, -32603]
```

**2. Enhanced "echo" method:**
```toml
name = "echo"
description = "Echo back the provided message"
params with minLength/maxLength validation
result with full object schema
```

#### Example Topics

**Enhanced "events.user" topic:**
```toml
name = "events.user"
description = "User-related events"
message schema with enum validation
pattern_type = "exact"
```

### Breaking Changes

**None** - All new fields are `Option<T>`, maintaining backwards compatibility.

Old configurations will continue to work and generate generic schemas. New fields can be added incrementally.

### Migration Path

#### Level 1: Minimal (Use existing config)
```toml
# Old style still works
[[asyncapi.methods]]
name = "myMethod"
example_params = '{"param": "value"}'
example_result = '"result"'
```

#### Level 2: Add Type Information
```toml
[[asyncapi.methods]]
name = "myMethod"
description = "Method description"
params_type = "object"
result_type = "string"
tags = ["category"]
```

#### Level 3: Full Schema
```toml
[[asyncapi.methods]]
name = "myMethod"
description = "Method description"
params_type = "object"
params_required = ["field1"]
params_properties = """
{
  "field1": {
    "type": "string",
    "minLength": 1,
    "maxLength": 100
  }
}
"""
result_schema = """
{
  "type": "object",
  "properties": {
    "result": {"type": "string"}
  }
}
"""
error_codes = [-32602, -32603]
tags = ["category"]
```

### Generated Output Improvements

With the new schema fields, the generated AsyncAPI specification includes:

1. **Method-Specific Messages**: Each method gets dedicated request/response messages
2. **Full Validation Rules**: minLength, maxLength, pattern, minimum, maximum, etc.
3. **Error Documentation**: Complete error catalog with descriptions
4. **Type-Safe Schemas**: Full JSON Schema for all parameters and results
5. **Better Examples**: Multiple examples with detailed context

### Code Generation Benefits

The enhanced schemas enable:
- **TypeScript**: Full type definitions with validation
- **Python**: Type hints and Pydantic models
- **Rust**: Strongly-typed structs with serde
- **Go**: Struct definitions with json tags
- **Java**: POJOs with Bean Validation

### Testing

Compilation verified with:
```bash
cd tools/template-gen
cargo check
# ✅ Finished successfully
```

### Documentation Updates

- Updated module documentation to mention "Schema-First Edition"
- Added section on schema-first features
- Updated configuration description
- Added notes about type safety and code generation

### Files Modified

- `tools/template-gen/src/main.rs` - Core generator code
- Default configuration with rich examples
- All struct definitions enhanced

### Compatibility

- ✅ **Backwards Compatible**: Old configs still work
- ✅ **Incremental Adoption**: Add new fields gradually
- ✅ **Default Values**: Sensible defaults for new fields
- ✅ **Optional Fields**: All new fields are `Option<T>`

### Next Steps for Users

1. **Regenerate config**: Run `cargo run` to get updated template
2. **Add schemas gradually**: Start with `params_type` and `result_type`
3. **Add validation**: Include `params_properties` for validation rules
4. **Define errors**: Add error code catalog
5. **Test generation**: Generate AsyncAPI and validate
6. **Generate SDKs**: Use AsyncAPI tools to generate type-safe clients

### Example Usage

```bash
# Generate new default config
cd tools/template-gen
cargo run

# Edit jrow-template.toml with your API definition

# Generate deployment files with enhanced AsyncAPI
cargo run

# Output includes schema-first AsyncAPI 3.0.0 spec
# deploy/asyncapi.yaml
```

### Benefits Summary

| Aspect | Before | After |
|--------|--------|-------|
| Type Safety | Minimal | Full JSON Schema |
| Validation | None | Complete rules |
| Errors | Generic | Cataloged with descriptions |
| Code Gen | Basic | Type-safe SDKs |
| Documentation | Simple | Comprehensive |
| Maintenance | Manual | Schema-driven |

---

**Version:** Schema-First v1.0  
**Date:** 2025-12-27  
**AsyncAPI:** 3.0.0  
**Rust Version:** 1.75+


