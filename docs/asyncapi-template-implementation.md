# AsyncAPI Template Implementation

## Overview

Added a Tera-based template for generating customized AsyncAPI 3.0 specifications for JROW-based projects. This allows users to document their custom RPC methods, pub/sub topics, and server configurations.

## Implementation

### 1. Template File

**File**: `templates/asyncapi.yaml.tera`

A comprehensive AsyncAPI 3.0 specification template with Tera variables for customization:

**Customizable Elements:**
- Project information (name, version, description, license, contact)
- Server configurations (production/development hosts, ports, protocols)
- Security settings (enable/disable security schemes)
- Custom RPC methods with example parameters and results
- Custom pub/sub topics with example data

**Key Features:**
- Full JSON-RPC 2.0 message schemas
- Request/Response, Notification, and Batch operations
- Pub/Sub operations (subscribe, unsubscribe, topic notifications)
- Conditional rendering (security schemes, optional fields)
- Loop support for methods and topics arrays

### 2. Configuration Schema

**File**: `jrow-template.toml.example` (updated)

Added `[asyncapi]` section with:

```toml
[asyncapi]
production_host = "api.example.com"
production_port = 443
production_protocol = "wss"
development_host = "localhost"
development_port = 8080
development_protocol = "ws"
security_enabled = true

[[asyncapi.methods]]
name = "add"
example_params = '{"a": 5, "b": 3}'
example_result = "8"

[[asyncapi.topics]]
name = "stock.prices"
example_params = '{"symbol": "AAPL", "price": 150.0}'
```

### 3. Template Generator Updates

**File**: `tools/template-gen/src/main.rs`

**New Structs:**
```rust
#[derive(Debug, Serialize, Deserialize)]
struct AsyncApiConfig {
    production_host: String,
    production_port: u16,
    production_protocol: String,
    development_host: String,
    development_port: u16,
    development_protocol: String,
    security_enabled: bool,
    methods: Vec<RpcMethod>,
    topics: Vec<PubSubTopic>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcMethod {
    name: String,
    example_params: Option<String>,
    example_result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PubSubTopic {
    name: String,
    example_params: Option<String>,
}
```

**Changes:**
- Added `asyncapi` field to `TemplateConfig`
- Updated `ProjectConfig` with license and contact fields
- Added AsyncAPI template to embedded templates list
- Updated context insertion to include `asyncapi` config
- Added AsyncAPI validation step to output instructions
- Removed unused `HashMap` import

### 4. Documentation Updates

**Files Updated:**
- `templates/README.md` - Comprehensive AsyncAPI template documentation
- `README.md` - Updated API Documentation section with template usage
- `jrow-template.toml.example` - Added AsyncAPI configuration examples

**Key Documentation Sections:**
- Configuration guide for AsyncAPI settings
- Usage examples for defining methods and topics
- AsyncAPI CLI commands for validation and generation
- Integration with AsyncAPI Studio
- Code generation examples

## Usage Workflow

### 1. Configure Your API

Edit `jrow-template.toml`:

```toml
[project]
name = "my-api"
version = "1.0.0"
license = "MIT"

[asyncapi]
production_host = "api.myapp.com"
production_port = 443
production_protocol = "wss"

[[asyncapi.methods]]
name = "getUserProfile"
example_params = '{"userId": "123"}'
example_result = '{"id": "123", "name": "Alice"}'

[[asyncapi.topics]]
name = "chat.messages"
example_params = '{"user": "alice", "message": "Hello"}'
```

### 2. Generate Files

```bash
make template-generate
```

Generates:
- `deploy/asyncapi.yaml` - Customized AsyncAPI specification
- `deploy/docker/Dockerfile` - Docker configuration
- `deploy/k8s/*.yaml` - Kubernetes manifests
- `deploy/README.md` - Deployment guide

### 3. Use AsyncAPI Tools

```bash
# Validate
asyncapi validate deploy/asyncapi.yaml

# Generate HTML docs
asyncapi generate fromTemplate deploy/asyncapi.yaml @asyncapi/html-template -o docs/

# Start AsyncAPI Studio
asyncapi start studio deploy/asyncapi.yaml

# Generate client SDK
asyncapi generate fromTemplate deploy/asyncapi.yaml @asyncapi/nodejs-template -o client/
```

## Benefits

1. **Customizable Documentation**: Each project can document its specific API
2. **Code Generation**: Generate client SDKs in multiple languages
3. **Interactive Docs**: View and edit in AsyncAPI Studio
4. **Validation**: Ensure API messages match the specification
5. **Industry Standard**: AsyncAPI 3.0 is widely supported
6. **Automated**: Generate from configuration, no manual YAML editing

## Template Features

### Dynamic Content

- **Methods Loop**: Iterates through `asyncapi.methods` array
- **Topics Loop**: Iterates through `asyncapi.topics` array
- **Conditional Security**: Shows security schemes only if enabled
- **Optional Fields**: Handles optional license_url, contact_name, etc.

### Example Rendering

Input configuration:
```toml
[[asyncapi.methods]]
name = "add"
example_params = '{"a": 5, "b": 3}'
example_result = "8"
```

Rendered output:
```yaml
examples:
  - jsonrpc: "2.0"
    method: "add"
    params: {"a": 5, "b": 3}
    id: 1
```

## Files Modified

1. `templates/asyncapi.yaml.tera` - New template file
2. `jrow-template.toml.example` - Added AsyncAPI config
3. `tools/template-gen/src/main.rs` - Added AsyncAPI support
4. `templates/README.md` - Comprehensive documentation
5. `README.md` - Updated API Documentation section

## Testing

Tested with:
```bash
make template-init
make template-generate
ls -la deploy/asyncapi.yaml  # Verify file exists
head -30 deploy/asyncapi.yaml  # Verify content
```

Results:
- ✅ AsyncAPI file generated successfully
- ✅ All variables rendered correctly
- ✅ Methods and topics arrays populated
- ✅ Conditional security rendering works
- ✅ No compilation warnings

## Future Enhancements

Potential improvements:
- [ ] AsyncAPI validation in template generator
- [ ] Support for method parameters schema definitions
- [ ] Support for topic message schema definitions
- [ ] Custom error codes documentation
- [ ] Authentication flow examples
- [ ] WebSocket channel bindings
- [ ] Message traits and reusable components

## Related Files

- `templates/asyncapi.yaml` - Static spec for JROW framework
- `templates/asyncapi.yaml.tera` - Template for user projects
- `templates/.asyncapi-studio` - AsyncAPI Studio config
- `Makefile` - Build targets for AsyncAPI operations

