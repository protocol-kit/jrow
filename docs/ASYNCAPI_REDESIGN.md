# AsyncAPI Template Redesign - Schema-First Approach

## Overview

This document explains the **Schema-First with Full Type Safety** redesign of the AsyncAPI template for JROW.

## Key Improvements

### 1. **Rich Schema Definitions**

Every method now includes complete JSON Schema for:
- **Parameters**: Full type information with validation rules
- **Results**: Structured response schemas
- **Errors**: Documented error codes with descriptions

**Example Configuration:**

```toml
[[asyncapi.methods]]
name = "getUserProfile"
description = "Get user profile information by user ID"
params_type = "object"
params_required = ["userId"]
params_properties = """
{
  "userId": {
    "type": "string",
    "pattern": "^[a-zA-Z0-9-]+$",
    "minLength": 1,
    "maxLength": 64
  },
  "includePrivate": {
    "type": "boolean",
    "default": false
  }
}
"""
result_schema = """
{
  "type": "object",
  "required": ["userId", "username", "createdAt"],
  "properties": {
    "userId": {"type": "string"},
    "username": {"type": "string"},
    "email": {"type": "string", "format": "email"},
    "createdAt": {"type": "string", "format": "date-time"}
  }
}
"""
```

### 2. **Method-Specific Messages**

Each method now generates:
- Dedicated request message with typed parameters
- Dedicated response message with typed results
- Specific examples for that method
- Associated error codes

**Generated Output:**

```yaml
AddRequest:
  name: add Request
  title: Add two numbers together
  payload:
    type: object
    required: [jsonrpc, method, id]
    properties:
      params:
        type: object
        required: [a, b]
        properties:
          a:
            type: number
            description: First operand
          b:
            type: number
            description: Second operand
```

### 3. **Error Code Catalog**

Complete error documentation with:
- Standard JSON-RPC error codes (-32700 to -32603)
- Application-specific error codes
- Error descriptions and use cases
- Per-method error associations

**Benefits:**
- Clients know exactly what errors to expect
- Better error handling in generated SDKs
- Clear API contract

### 4. **Topic Schemas**

Pub/Sub topics now include:
- Full message schemas with validation
- Required fields specification
- Field-level descriptions
- Pattern types (exact vs wildcard)
- Publish rate information

**Example:**

```toml
[[asyncapi.topics]]
name = "stock.prices"
description = "Real-time stock price updates"
message_type = "object"
message_required = ["symbol", "price", "timestamp"]
message_properties = """
{
  "symbol": {
    "type": "string",
    "pattern": "^[A-Z]{1,5}$"
  },
  "price": {
    "type": "number",
    "minimum": 0,
    "exclusiveMinimum": true
  },
  "timestamp": {
    "type": "string",
    "format": "date-time"
  }
}
"""
publish_rate = "High (multiple per second)"
```

### 5. **Validation Rules**

Schemas include JSON Schema validation:
- **String validation**: minLength, maxLength, pattern, format, enum
- **Number validation**: minimum, maximum, exclusiveMinimum
- **Array validation**: minItems, maxItems, items schema
- **Object validation**: required fields, additional properties

### 6. **Enhanced Documentation**

Every element includes:
- Detailed descriptions
- Multiple examples
- Usage guidelines
- Rate limiting information
- Connection limits
- Batch execution mode

### 7. **Better Code Generation Support**

With complete type information, tools can generate:
- **Type-safe client SDKs** in TypeScript, Python, Rust, Go, etc.
- **Validation middleware** that checks requests/responses
- **Mock servers** with realistic data
- **Interactive documentation** with working examples

## Migration Guide

### From Old Template

**Old (minimal schema):**

```toml
[[asyncapi.methods]]
name = "add"
example_params = '{"a": 5, "b": 3}'
example_result = "8"
```

**New (full schema):**

```toml
[[asyncapi.methods]]
name = "add"
description = "Add two numbers together"
tags = ["math", "calculation"]
params_type = "object"
params_required = ["a", "b"]
params_properties = """
{
  "a": {"type": "number", "description": "First operand"},
  "b": {"type": "number", "description": "Second operand"}
}
"""
result_type = "number"
result_description = "Sum of a and b"
example_params = '{"a": 5, "b": 3}'
example_result = "8"
error_codes = [-32602, -32603]
```

## Template Structure

### New Sections

1. **Error Code Definitions** (`asyncapi.error_codes`)
   - Standard JSON-RPC errors
   - Application-specific errors
   - Complete error catalog

2. **Rich Method Definitions** (`asyncapi.methods`)
   - Full parameter schemas
   - Full result schemas
   - Error associations
   - Tags and metadata
   - Deprecation flags

3. **Rich Topic Definitions** (`asyncapi.topics`)
   - Complete message schemas
   - Pattern types
   - Publish rates
   - Field validation

4. **Method-Specific Messages**
   - Individual request messages per method
   - Individual response messages per method
   - Typed parameters and results

5. **Topic-Specific Notifications**
   - Individual notification messages per topic
   - Typed payloads

## Benefits Summary

### For Developers
- ✅ Complete type information upfront
- ✅ Better IDE autocomplete and validation
- ✅ Clearer API contracts
- ✅ Fewer runtime errors

### For Code Generators
- ✅ Can generate type-safe clients
- ✅ Can generate validation code
- ✅ Can generate comprehensive mocks
- ✅ Better documentation generation

### For API Consumers
- ✅ Self-documenting API
- ✅ Clear error handling
- ✅ Interactive documentation
- ✅ Easier testing

### For Maintainers
- ✅ Schema as single source of truth
- ✅ Better breaking change detection
- ✅ Easier versioning
- ✅ Clear deprecation paths

## Example Use Cases

### 1. Generate TypeScript SDK

With full schemas, tools like `@asyncapi/modelina` can generate:

```typescript
interface AddRequest {
  jsonrpc: "2.0";
  method: "add";
  params: {
    a: number;
    b: number;
  };
  id: string | number;
}

interface AddResponse {
  jsonrpc: "2.0";
  result: number;
  id: string | number;
}

type AddError = 
  | { code: -32602; message: "Invalid method parameter(s)" }
  | { code: -32603; message: "Internal JSON-RPC error" };
```

### 2. Validate Requests

```rust
// Generated validation code
fn validate_add_params(params: &Value) -> Result<(), ValidationError> {
    let obj = params.as_object().ok_or(ValidationError::NotObject)?;
    
    let a = obj.get("a").ok_or(ValidationError::MissingField("a"))?;
    let b = obj.get("b").ok_or(ValidationError::MissingField("b"))?;
    
    validate_number(a)?;
    validate_number(b)?;
    
    Ok(())
}
```

### 3. Generate Documentation

AsyncAPI Studio and other tools will show:
- Complete method signatures
- Parameter types and constraints
- Return types
- All possible errors
- Interactive examples

## Configuration Best Practices

### 1. Always Specify Types

```toml
# Good
params_type = "object"
result_type = "number"

# Avoid (falls back to generic schemas)
# No type specification
```

### 2. Include Validation Rules

```toml
params_properties = """
{
  "email": {
    "type": "string",
    "format": "email",
    "minLength": 3,
    "maxLength": 255
  },
  "age": {
    "type": "integer",
    "minimum": 0,
    "maximum": 150
  }
}
"""
```

### 3. Document All Fields

```toml
params_properties = """
{
  "userId": {
    "type": "string",
    "description": "Unique user identifier",  # Always add descriptions
    "examples": ["user-123", "abc-def"]
  }
}
"""
```

### 4. Associate Error Codes

```toml
error_codes = [-32602, -32603, -32001]  # Reference error_codes section
```

### 5. Tag Your Methods and Topics

```toml
tags = ["user", "profile", "authenticated"]  # Helps with organization
```

## File Structure

```
templates/
├── jrow-template.toml          # Enhanced configuration
├── asyncapi.yaml.tera          # Schema-first template
└── ASYNCAPI_REDESIGN.md        # This file
```

## Testing the Template

### 1. Copy and Customize Config

```bash
cp templates/jrow-template.toml ./jrow-config.toml
# Edit jrow-config.toml with your methods and topics
```

### 2. Generate AsyncAPI Spec

```bash
# Using your template engine
tera render templates/asyncapi.yaml.tera jrow-config.toml > asyncapi.yaml
```

### 3. Validate with AsyncAPI CLI

```bash
npm install -g @asyncapi/cli
asyncapi validate asyncapi.yaml
```

### 4. Generate Documentation

```bash
asyncapi generate fromTemplate asyncapi.yaml @asyncapi/html-template -o docs/
```

### 5. Generate Client SDK

```bash
asyncapi generate fromTemplate asyncapi.yaml @asyncapi/ts-nats-template -o client/
```

## Backwards Compatibility

The template maintains backwards compatibility:

- **Old config** (minimal): Still works, generates generic schemas
- **New config** (rich schemas): Generates full type-safe schemas
- **Mixed config**: Can gradually add schemas to existing methods

## Future Enhancements

Potential additions:
- [ ] OpenAPI 3.1 compatibility mode
- [ ] GraphQL schema generation
- [ ] Protobuf definitions
- [ ] More code generation targets
- [ ] Schema versioning support
- [ ] Breaking change detection
- [ ] API diff tools

## Questions?

See the example configuration in `templates/jrow-template.toml` for complete examples of:
- Math operations with simple types
- User operations with complex objects
- Search operations with pagination
- Stock prices with real-time updates
- Chat messages with rich schemas
- System alerts with enums

## Summary

This Schema-First redesign transforms the AsyncAPI template from basic documentation into a comprehensive, type-safe API specification that enables:

1. **Better tooling** - Code generation, validation, mocking
2. **Clearer contracts** - Complete type information upfront
3. **Fewer bugs** - Validation at compile-time and runtime
4. **Better DX** - IDE support, autocomplete, inline docs
5. **Easier maintenance** - Schema as single source of truth

The investment in detailed schemas pays off through better code generation, fewer integration issues, and clearer API contracts.


