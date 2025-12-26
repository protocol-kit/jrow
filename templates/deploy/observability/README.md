# JROW Observability Stack

This directory contains the observability stack deployment for JROW applications.

## Components

- **Jaeger**: Distributed tracing backend and UI
- **Prometheus**: Metrics storage and querying
- **Grafana**: Visualization and dashboards

## Quick Start

```bash
# Start the observability stack
docker-compose -f docker-compose.observability.yml up -d

# Check status
docker-compose -f docker-compose.observability.yml ps

# View logs
docker-compose -f docker-compose.observability.yml logs -f

# Stop the stack
docker-compose -f docker-compose.observability.yml down
```

## Access URLs

- **Jaeger UI**: http://localhost:16686
  - View distributed traces
  - Search by service name, operation, tags
  - Analyze request latencies

- **Prometheus**: http://localhost:9090
  - Query metrics
  - Create custom queries
  - View targets and alerts

- **Grafana**: http://localhost:3000
  - Login: admin/admin
  - Pre-configured Prometheus and Jaeger data sources
  - Create custom dashboards

## OTLP Endpoints

Your JROW application should connect to:
- **gRPC**: `http://localhost:4317`
- **HTTP**: `http://localhost:4318`

## Configuration

### Prometheus (`prometheus.yml`)

Configure scrape targets for your JROW application metrics:

```yaml
scrape_configs:
  - job_name: 'jrow-server'
    static_configs:
      - targets: ['host.docker.internal:8080']
```

### Grafana (`grafana-datasources.yml`)

Data sources are pre-configured:
- Prometheus (default)
- Jaeger

## Data Persistence

Data is persisted in Docker volumes:
- `prometheus-data`: Metrics time series
- `grafana-data`: Dashboards and settings

To remove all data:
```bash
docker-compose -f docker-compose.observability.yml down -v
```

## Production Deployment

For production, consider:

1. **External OTLP Collector**: Use OpenTelemetry Collector for better performance
2. **Persistent Storage**: Use external volumes or cloud storage
3. **High Availability**: Run multiple instances with load balancing
4. **Security**: Enable authentication, TLS, and access controls
5. **Resource Limits**: Set appropriate CPU and memory limits
6. **Retention**: Configure data retention policies

## Integration with JROW

### Server

```rust
use jrow_core::ObservabilityConfig;
use jrow_server::ServerBuilder;

let config = ObservabilityConfig::new("my-service")
    .with_endpoint("http://localhost:4317");

let server = ServerBuilder::new()
    .bind(addr)
    .with_observability(config)
    .build()
    .await?;
```

### Client

```rust
use jrow_client::ClientBuilder;
use jrow_core::ObservabilityConfig;

let config = ObservabilityConfig::new("my-client")
    .with_endpoint("http://localhost:4317");

let client = ClientBuilder::new(url)
    .with_observability(config)
    .connect()
    .await?;
```

## Troubleshooting

### No traces appearing in Jaeger

1. Check Jaeger is running: `docker ps | grep jaeger`
2. Verify OTLP endpoint is accessible
3. Check application logs for connection errors
4. Ensure `COLLECTOR_OTLP_ENABLED=true` in Jaeger config

### Prometheus not scraping metrics

1. Check Prometheus targets: http://localhost:9090/targets
2. Verify scrape configuration in `prometheus.yml`
3. Ensure your application exposes metrics endpoint
4. Check network connectivity

### Grafana can't connect to data sources

1. Verify Prometheus and Jaeger are running
2. Check data source URLs in Grafana settings
3. Test connection in Grafana UI
4. Check docker network connectivity

## Example Queries

### Prometheus Queries

```promql
# Request rate
rate(jrow_server_requests_total[5m])

# Request duration 95th percentile
histogram_quantile(0.95, rate(jrow_server_request_duration_seconds_bucket[5m]))

# Active connections
jrow_server_connections_active

# Error rate
rate(jrow_server_errors_total[5m])
```

### Jaeger Searches

- Service: `my-service`
- Operation: `rpc_request`
- Tags: `method=add`, `status=success`
- Min/Max Duration filters

## See Also

- [docs/observability.md](../../../docs/observability.md) - Full observability documentation
- [JROW Examples](../../../examples/) - Example applications with observability

