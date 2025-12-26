# JROW Templates

This directory contains templates for the JROW project and projects built with JROW.

## Directory Structure

```
templates/
├── asyncapi.yaml.tera     # AsyncAPI 3.0 specification template
├── asyncapi.yaml          # Static AsyncAPI spec (for JROW itself)
├── .asyncapi-studio       # AsyncAPI Studio configuration
├── deploy/                # Deployment template files (Tera templates)
│   ├── docker/
│   │   ├── Dockerfile.tera
│   │   └── docker-compose.yml.tera
│   ├── k8s/
│   │   ├── deployment.yaml.tera
│   │   └── configmap.yaml.tera
│   ├── scripts/
│   │   ├── deploy.sh        # Static deployment script (for JROW itself)
│   │   └── deploy.sh.tera   # Deployment script template
│   └── README.md.tera
└── README.md             # This file
```

## Contents

### AsyncAPI Specification

- **`asyncapi.yaml`**: Static AsyncAPI 3.0 specification for the JROW framework itself
- **`asyncapi.yaml.tera`**: Customizable AsyncAPI template for your JROW-based projects

The template allows you to define your custom RPC methods, pub/sub topics, and server configurations.

See the AsyncAPI section below for usage details.

### Deployment Templates (Tera)

The `deploy/` directory contains **Tera templates** for generating deployment configurations for projects built with JROW.

**Important**: These are templates, not ready-to-use deployment files. They need to be rendered with your project's configuration.

## Using Deployment Templates

### 1. Install Template Generator

```bash
# Build and install the template generator tool
make template-gen-build
make template-gen-install
```

Or manually:

```bash
cd tools/template-gen
cargo build --release
cargo install --path .
```

### 2. Initialize Configuration

```bash
# Copy the template configuration
cp templates/jrow-template.toml jrow-template.toml

# Or use make
make template-init
```

### 3. Edit Configuration

Edit `jrow-template.toml` with your project details:

```toml
[project]
name = "my-jrow-app"
description = "My WebSocket RPC service"
version = "0.1.0"
license = "MIT"

[server]
bind_address = "0.0.0.0"
port = 8080
batch_mode = "Parallel"

[docker]
image_name = "my-jrow-app"
registry = "docker.io/myuser"
expose_ports = [8080]

[kubernetes]
namespace = "production"
replicas = 3
service_type = "LoadBalancer"

[asyncapi]
production_host = "api.example.com"
production_port = 443
production_protocol = "wss"
development_host = "localhost"
development_port = 8080
development_protocol = "ws"
security_enabled = true

# Define your RPC methods
[[asyncapi.methods]]
name = "add"
example_params = '{"a": 5, "b": 3}'
example_result = "8"

[[asyncapi.methods]]
name = "getUserProfile"
example_params = '{"userId": "123"}'
example_result = '{"id": "123", "name": "Alice"}'

# Define your pub/sub topics
[[asyncapi.topics]]
name = "stock.prices"
example_params = '{"symbol": "AAPL", "price": 150.0}'

[[asyncapi.topics]]
name = "chat.messages"
example_params = '{"user": "alice", "message": "Hello"}'
```

### 4. Generate Deployment Files

```bash
# Generate deployment files
make template-generate

# Or run the tool directly
jrow-template-gen -c jrow-template.toml -o deploy
```

This will create a `deploy/` directory with:
- `deploy/docker/Dockerfile` - Your customized Dockerfile
- `deploy/docker/docker-compose.yml` - Docker Compose config
- `deploy/k8s/deployment.yaml` - Kubernetes Deployment and Service
- `deploy/k8s/configmap.yaml` - Kubernetes ConfigMap
- `deploy/scripts/deploy.sh` - Deployment script (executable)
- `deploy/README.md` - Deployment documentation
- `deploy/asyncapi.yaml` - Your customized AsyncAPI specification

### 5. Deploy Your Application

```bash
# Use the generated deployment script
cd deploy
./scripts/deploy.sh docker      # Deploy with Docker
./scripts/deploy.sh k8s          # Deploy to Kubernetes
./scripts/deploy.sh build        # Build release binary
./scripts/deploy.sh run          # Run locally
./scripts/deploy.sh status       # Check status
./scripts/deploy.sh help         # Show all commands

# Or use make targets from project root
make deploy-docker
make deploy-k8s
```

## Template Variables

The templates support the following variables:

### Project
- `project.name` - Project name
- `project.description` - Project description
- `project.version` - Version
- `project.rust_version` - Rust version for Docker

### Server
- `server.bind_address` - Bind address
- `server.port` - Port number
- `server.batch_mode` - Batch processing mode
- `server.max_connections` - Max connections
- `server.connection_timeout` - Timeout in seconds

### Docker
- `docker.image_name` - Docker image name
- `docker.registry` - Docker registry (optional)
- `docker.expose_ports` - Array of ports to expose

### Kubernetes
- `kubernetes.namespace` - K8s namespace
- `kubernetes.replicas` - Number of replicas
- `kubernetes.service_type` - Service type (LoadBalancer/ClusterIP/NodePort)
- `kubernetes.resources.*` - CPU and memory requests/limits

### AsyncAPI
- `asyncapi.production_host` - Production server host
- `asyncapi.production_port` - Production server port
- `asyncapi.production_protocol` - Production protocol (wss/ws)
- `asyncapi.development_host` - Development server host
- `asyncapi.development_port` - Development server port
- `asyncapi.development_protocol` - Development protocol (ws)
- `asyncapi.security_enabled` - Enable security schemes
- `asyncapi.methods` - Array of RPC methods with examples
- `asyncapi.topics` - Array of pub/sub topics with examples

## Deployment Script

The generated `deploy/scripts/deploy.sh` is a comprehensive deployment script with:

**Commands:**
- `docker` - Deploy using Docker Compose
- `k8s` - Deploy to Kubernetes cluster
- `build` - Build release binary
- `run` - Run server locally with cargo
- `push` - Push Docker image to registry (if configured)
- `status` - Check deployment status
- `cleanup` - Clean up Docker/Kubernetes resources

**Features:**
- Colored output for better readability
- Error handling and validation
- Automatic namespace creation for Kubernetes
- Optional image and namespace cleanup
- Environment variable configuration
- Help documentation

**Example Usage:**
```bash
# Deploy to Docker
./deploy/scripts/deploy.sh docker

# Check status
./deploy/scripts/deploy.sh status docker

# Deploy to Kubernetes
./deploy/scripts/deploy.sh k8s

# Clean up
./deploy/scripts/deploy.sh cleanup docker
```

## AsyncAPI Specification

### Using the AsyncAPI Template

The `asyncapi.yaml.tera` template allows you to generate customized API documentation for your JROW-based project.

**Steps:**

1. **Configure your API** in `jrow-template.toml` (see example above)
2. **Generate the spec**: `make template-generate`
3. **Use the generated spec**: `deploy/asyncapi.yaml`

### View in AsyncAPI Studio

**For JROW framework itself:**
1. Go to [AsyncAPI Studio](https://studio.asyncapi.com/)
2. Import `templates/asyncapi.yaml`
3. Explore the interactive documentation

**For your generated spec:**
1. Go to [AsyncAPI Studio](https://studio.asyncapi.com/)
2. Import `deploy/asyncapi.yaml`
3. Explore your customized API documentation

### Generate Documentation

```bash
# Install AsyncAPI CLI
npm install -g @asyncapi/cli

# Validate your generated spec
asyncapi validate deploy/asyncapi.yaml

# Generate HTML documentation
asyncapi generate fromTemplate deploy/asyncapi.yaml @asyncapi/html-template -o docs/

# Generate Markdown documentation
asyncapi generate fromTemplate deploy/asyncapi.yaml @asyncapi/markdown-template -o docs/

# Or use make targets (for JROW framework spec)
make asyncapi-html
make asyncapi-md
make asyncapi-validate
```

### What's Documented

- Core JSON-RPC 2.0 operations (request/response, notifications, batches)
- Pub/Sub operations (subscribe, unsubscribe, topic notifications)
- Batch operations (batch subscribe/unsubscribe, batch publish)
- Your custom RPC methods with example parameters and results
- Your custom pub/sub topics with example data
- Message formats and schemas
- Server definitions (production and development)
- Security schemes (Bearer token, API key)

## For JROW Contributors

When updating JROW features:

1. **AsyncAPI**: Update both `asyncapi.yaml` (static) and `asyncapi.yaml.tera` (template) with new operations
2. **Templates**: Update `.tera` files if deployment needs change
3. **Generator**: Update `tools/template-gen` if new config options needed
4. **Validate**: Run `asyncapi validate templates/asyncapi.yaml`
5. **Test**: Generate templates with `make template-generate` and test deployment
6. **Document**: Update this README

### Maintaining Two AsyncAPI Files

- **`asyncapi.yaml`**: Static specification for JROW framework itself (for documentation)
- **`asyncapi.yaml.tera`**: Template for user projects (for generation)

When adding new features, update both files to keep them in sync.

## Tools and Resources

### Tera Template Engine
- [Tera Documentation](https://tera.netlify.app/)
- [Template Syntax](https://tera.netlify.app/docs/#templates)

### AsyncAPI
- [AsyncAPI Studio](https://studio.asyncapi.com/)
- [AsyncAPI CLI](https://github.com/asyncapi/cli)
- [AsyncAPI Generator](https://github.com/asyncapi/generator)
- [AsyncAPI Specification](https://www.asyncapi.com/docs/reference/specification/latest)

### Deployment
- [Docker Documentation](https://docs.docker.com/)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Docker Compose](https://docs.docker.com/compose/)

## License

Same as the main JROW project (MIT).
