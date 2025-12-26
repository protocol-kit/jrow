# JROW Deployment

This directory contains deployment configurations and scripts for the JROW (JSON-RPC over WebSocket) toolkit.

## Directory Structure

```
templates/deploy/
├── docker/           # Docker and Docker Compose configurations
│   ├── Dockerfile              # Production multi-stage build
│   ├── Dockerfile.dev          # Development with hot reload
│   └── docker-compose.yml      # Multi-service composition
├── k8s/              # Kubernetes manifests
│   ├── deployment.yaml         # Deployment and Service
│   └── configmap.yaml          # Configuration
├── scripts/          # Deployment scripts
│   └── deploy.sh               # Unified deployment script
└── README.md         # This file
```

## Quick Start

### Using the Deploy Script

The easiest way to deploy is using the unified deployment script:

```bash
# Deploy with Docker
./templates/deploy/scripts/deploy.sh docker

# Deploy to Kubernetes
./templates/deploy/scripts/deploy.sh k8s

# Run locally
./templates/deploy/scripts/deploy.sh run

# Run specific example locally
./templates/deploy/scripts/deploy.sh run pubsub

# Clean up
./templates/deploy/scripts/deploy.sh cleanup docker
./templates/deploy/scripts/deploy.sh cleanup k8s
```

## Docker Deployment

### Production Build

Build and run the production Docker image:

```bash
# Build the image
docker build -t jrow:latest -f templates/deploy/docker/Dockerfile .

# Run the container
docker run -p 8080:8080 jrow:latest

# Or run a specific example
docker run -p 9000:9000 jrow:latest pubsub
```

### Docker Compose

Run multiple services with Docker Compose:

```bash
cd templates/deploy/docker
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

### Development with Hot Reload

For development with automatic reloading:

```bash
docker build -t jrow:dev -f templates/deploy/docker/Dockerfile.dev .
docker run -v $(pwd):/usr/src/jrow -p 8080:8080 jrow:dev
```

## Kubernetes Deployment

### Prerequisites

- `kubectl` installed and configured
- Access to a Kubernetes cluster
- Docker image built and pushed to a registry (for production)

### Deploy to Kubernetes

```bash
# Apply configurations
kubectl apply -f templates/deploy/k8s/configmap.yaml
kubectl apply -f templates/deploy/k8s/deployment.yaml

# Check deployment status
kubectl get deployments
kubectl get pods
kubectl get services

# View logs
kubectl logs -f deployment/jrow-server

# Get service endpoint
kubectl get service jrow-server
```

### Configuration

Edit `templates/deploy/k8s/configmap.yaml` to customize server settings:

- `bind_address`: Server bind address
- `batch_mode`: Batch processing mode (Parallel/Sequential)
- `rust_log`: Logging level
- `max_connections`: Maximum concurrent connections
- `connection_timeout`: Connection timeout in seconds

### Scaling

Scale the deployment:

```bash
# Scale to 5 replicas
kubectl scale deployment jrow-server --replicas=5

# Auto-scale based on CPU
kubectl autoscale deployment jrow-server --min=2 --max=10 --cpu-percent=80
```

## Environment Variables

Common environment variables:

- `RUST_LOG`: Logging level (trace, debug, info, warn, error)
- `BIND_ADDRESS`: Server bind address (default: 0.0.0.0:8080)
- `BATCH_MODE`: Batch processing mode (Parallel or Sequential)

## Examples

### Simple Server

```bash
# Docker
docker run -p 8080:8080 jrow:latest simple_server

# Kubernetes
kubectl apply -f deploy/k8s/deployment.yaml
```

### Pub/Sub Server

```bash
# Docker
docker run -p 9000:9000 jrow:latest pubsub

# Docker Compose (runs automatically)
cd deploy/docker && docker-compose up pubsub-server
```

### Batch Examples

```bash
# Run batch example
docker run -p 9002:9002 jrow:latest batch

# Run pubsub_batch example
docker run -p 9003:9003 jrow:latest pubsub_batch

# Run publish_batch example
docker run -p 9004:9004 jrow:latest publish_batch
```

## Health Checks

The Kubernetes deployment includes:

- **Liveness Probe**: TCP socket check on port 8080
- **Readiness Probe**: TCP socket check on port 8080

For production use, you may want to implement HTTP health check endpoints.

## Resource Limits

Default resource limits in Kubernetes:

- **Requests**: 64Mi memory, 100m CPU
- **Limits**: 128Mi memory, 500m CPU

Adjust these in `templates/deploy/k8s/deployment.yaml` based on your workload.

## Monitoring

### Docker

View container logs:

```bash
docker logs -f <container_id>
docker-compose logs -f
```

### Kubernetes

View logs:

```bash
# All pods
kubectl logs -f -l app=jrow-server

# Specific pod
kubectl logs -f <pod-name>

# Previous container
kubectl logs --previous <pod-name>
```

Monitor resources:

```bash
# Resource usage
kubectl top pods
kubectl top nodes

# Events
kubectl get events --sort-by=.metadata.creationTimestamp
```

## Troubleshooting

### Docker

1. **Container won't start**:
   ```bash
   docker logs <container_id>
   ```

2. **Port already in use**:
   ```bash
   # Change port mapping
   docker run -p 8081:8080 jrow:latest
   ```

3. **Build failures**:
   ```bash
   # Clean build cache
   docker build --no-cache -t jrow:latest -f templates/deploy/docker/Dockerfile .
   ```

### Kubernetes

1. **Pods not starting**:
   ```bash
   kubectl describe pod <pod-name>
   kubectl logs <pod-name>
   ```

2. **Service not accessible**:
   ```bash
   kubectl get svc jrow-server
   kubectl describe svc jrow-server
   ```

3. **Image pull errors**:
   - Ensure image is pushed to registry
   - Check imagePullSecrets if using private registry

## Production Considerations

1. **Security**:
   - Use TLS/WSS in production
   - Implement authentication and authorization
   - Use secrets for sensitive configuration
   - Run as non-root user

2. **Performance**:
   - Adjust resource limits based on load
   - Use batch processing mode appropriately
   - Monitor memory usage and adjust limits
   - Consider horizontal pod autoscaling

3. **High Availability**:
   - Run multiple replicas
   - Use pod disruption budgets
   - Implement proper health checks
   - Use StatefulSet if you need stable network identities

4. **Monitoring & Logging**:
   - Integrate with Prometheus for metrics
   - Use centralized logging (ELK, Loki, etc.)
   - Set up alerts for critical issues
   - Monitor WebSocket connection counts

## CI/CD Integration

Example GitHub Actions workflow:

```yaml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Build Docker image
        run: docker build -t jrow:${{ github.sha }} -f templates/deploy/docker/Dockerfile .
      
      - name: Push to registry
        run: |
          echo "${{ secrets.DOCKER_PASSWORD }}" | docker login -u "${{ secrets.DOCKER_USERNAME }}" --password-stdin
          docker push jrow:${{ github.sha }}
      
      - name: Deploy to Kubernetes
        run: |
          kubectl set image deployment/jrow-server jrow-server=jrow:${{ github.sha }}
```

## License

Same as the main JROW project (MIT).

## Support

For issues or questions:
- Check the main README
- Review example code
- Open an issue on GitHub

