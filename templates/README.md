# JROW AsyncAPI Templates - Schema-First Edition

## Quick Start

### 1. Copy Configuration Template

```bash
cp templates/jrow-template.toml ./jrow-config.toml
```

### 2. Customize Your API

Edit `jrow-config.toml` with your methods and topics:

```toml
[[asyncapi.methods]]
name = "myMethod"
description = "What this method does"
params_type = "object"
params_required = ["field1", "field2"]
params_properties = """
{
  "field1": {"type": "string", "description": "..."},
  "field2": {"type": "number", "minimum": 0}
}
"""
result_type = "object"
result_schema = """
{
  "type": "object",
  "properties": {
    "result": {"type": "string"}
  }
}
"""
```

### 3. Generate AsyncAPI Specification

```bash
# Using your template engine (Tera/Jinja2/etc)
tera render templates/asyncapi.yaml.tera jrow-config.toml > asyncapi.yaml
```

### 4. Validate & Generate

```bash
# Validate the spec
asyncapi validate asyncapi.yaml

# Generate documentation
asyncapi generate fromTemplate asyncapi.yaml @asyncapi/html-template -o docs/

# Generate TypeScript client
asyncapi generate fromTemplate asyncapi.yaml @asyncapi/ts-nats-template -o client/
```

## What's New in Schema-First?

### 🎯 Full Type Safety

Every method includes complete JSON Schema definitions:
- ✅ Parameter types with validation rules
- ✅ Result types with nested structures
- ✅ Error code associations
- ✅ Field-level documentation

### 📋 Rich Documentation

Auto-generated docs include:
- ✅ Method descriptions and examples
- ✅ Parameter constraints
- ✅ Return type structures
- ✅ Error catalog
- ✅ Rate limiting info

### 🔧 Better Code Generation

Generate type-safe SDKs in:
- TypeScript, Python, Rust, Go, Java, and more
- Full IntelliSense/autocomplete support
- Compile-time type checking
- Runtime validation

## Files

| File | Purpose |
|------|---------|
| `jrow-template.toml` | Configuration template with examples |
| `asyncapi.yaml.tera` | Schema-first AsyncAPI template |
| `asyncapi.yaml` | Pre-generated example output |
| `ASYNCAPI_REDESIGN.md` | Complete redesign documentation |
| `SCHEMA_COMPARISON.md` | Before/after comparison |
| `VALIDATION_REPORT.md` | Template validation results |
| `README.md` | This file |

## Examples

### Simple Method

```toml
[[asyncapi.methods]]
name = "add"
description = "Add two numbers"
params_type = "object"
params_required = ["a", "b"]
params_properties = """
{
  "a": {"type": "number"},
  "b": {"type": "number"}
}
"""
result_type = "number"
example_params = '{"a": 5, "b": 3}'
example_result = "8"
```

### Complex Method

```toml
[[asyncapi.methods]]
name = "searchItems"
description = "Search with pagination and filters"
params_type = "object"
params_required = ["query"]
params_properties = """
{
  "query": {
    "type": "string",
    "minLength": 1,
    "maxLength": 200
  },
  "page": {
    "type": "integer",
    "minimum": 1,
    "default": 1
  },
  "filters": {
    "type": "object",
    "properties": {
      "category": {
        "type": "string",
        "enum": ["electronics", "books", "clothing"]
      }
    }
  }
}
"""
result_schema = """
{
  "type": "object",
  "required": ["items", "total"],
  "properties": {
    "items": {
      "type": "array",
      "items": {"type": "object"}
    },
    "total": {"type": "integer"}
  }
}
"""
```

### Pub/Sub Topic

```toml
[[asyncapi.topics]]
name = "stock.prices"
description = "Real-time stock updates"
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
    "minimum": 0
  },
  "timestamp": {
    "type": "string",
    "format": "date-time"
  }
}
"""
```

## Validation Rules

### Strings
```json
{
  "type": "string",
  "minLength": 1,
  "maxLength": 100,
  "pattern": "^[a-zA-Z0-9]+$",
  "format": "email"  // or "uri", "date-time", etc.
}
```

### Numbers
```json
{
  "type": "number",
  "minimum": 0,
  "maximum": 100,
  "exclusiveMinimum": true,
  "multipleOf": 0.01
}
```

### Arrays
```json
{
  "type": "array",
  "minItems": 1,
  "maxItems": 10,
  "items": {"type": "string"}
}
```

### Objects
```json
{
  "type": "object",
  "required": ["field1", "field2"],
  "properties": {
    "field1": {"type": "string"},
    "field2": {"type": "number"}
  },
  "additionalProperties": false
}
```

### Enums
```json
{
  "type": "string",
  "enum": ["option1", "option2", "option3"]
}
```

## Error Codes

Define error codes once, reference everywhere:

```toml
# Define error codes
[[asyncapi.error_codes]]
code = -32001
name = "Unauthorized"
message = "Authentication required"
description = "The request requires authentication"

# Reference in methods
[[asyncapi.methods]]
name = "getProfile"
error_codes = [-32602, -32001, -32603]  # InvalidParams, Unauthorized, InternalError
```

## Best Practices

### 1. Always Specify Types

```toml
# Good ✅
params_type = "object"
result_type = "number"

# Avoid ❌
# No type specification
```

### 2. Include Descriptions

```toml
description = "Clear description of what this method does"
params_properties = """
{
  "userId": {
    "type": "string",
    "description": "Unique user identifier"  # Always describe fields
  }
}
"""
```

### 3. Add Validation Rules

```toml
params_properties = """
{
  "email": {
    "type": "string",
    "format": "email",      # Validation
    "minLength": 3,         # Validation
    "maxLength": 255        # Validation
  }
}
"""
```

### 4. Tag Your APIs

```toml
tags = ["user", "profile", "authenticated"]  # Helps organization
```

### 5. Document Errors

```toml
error_codes = [-32602, -32603, -32001]  # List expected errors
```

## Migration from Old Template

### Level 1: Minimal (5 mins)
```toml
# Add basic fields
description = "..."
params_type = "object"
result_type = "string"
```

### Level 2: Typed (15 mins)
```toml
# Add schema structures
params_required = ["field1"]
params_properties = """{ "field1": {"type": "string"} }"""
result_schema = """{ "type": "object", ... }"""
```

### Level 3: Complete (30 mins)
```toml
# Add validation and documentation
params_properties = """
{
  "field1": {
    "type": "string",
    "minLength": 1,
    "maxLength": 100,
    "description": "Field description",
    "examples": ["example1", "example2"]
  }
}
"""
error_codes = [-32602, -32603]
tags = ["category"]
```

## Tools

### AsyncAPI CLI
```bash
npm install -g @asyncapi/cli

# Validate
asyncapi validate asyncapi.yaml

# Generate docs
asyncapi generate fromTemplate asyncapi.yaml @asyncapi/html-template -o ./docs

# Generate client
asyncapi generate fromTemplate asyncapi.yaml @asyncapi/ts-nats-template -o ./client
```

### AsyncAPI Studio
Online editor and validator:
https://studio.asyncapi.com/

### Code Generators
- TypeScript: `@asyncapi/ts-nats-template`
- Python: `@asyncapi/python-paho-template`
- Java: `@asyncapi/java-spring-template`
- More: https://www.asyncapi.com/tools/generator

## Support

- 📖 **Full Documentation:** [ASYNCAPI_REDESIGN.md](ASYNCAPI_REDESIGN.md)
- 🔄 **Before/After Guide:** [SCHEMA_COMPARISON.md](SCHEMA_COMPARISON.md)
- ✅ **Validation Report:** [VALIDATION_REPORT.md](VALIDATION_REPORT.md)
- 💬 **Issues:** [GitHub Issues](https://github.com/protocol-kit/jrow/issues)

## Features

| Feature | Support |
|---------|---------|
| JSON-RPC 2.0 | ✅ |
| Request/Response | ✅ |
| Notifications | ✅ |
| Batch requests | ✅ |
| Pub/Sub | ✅ |
| Full type safety | ✅ |
| Validation rules | ✅ |
| Error catalog | ✅ |
| Code generation | ✅ |
| AsyncAPI 3.0.0 | ✅ |

## License

Same as JROW project (see root LICENSE file).

---

**Version:** Schema-First v1.0  
**AsyncAPI:** 3.0.0  
**Updated:** 2025-12-27
