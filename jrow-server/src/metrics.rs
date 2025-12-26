//! Server metrics definitions
//!
//! This module defines OpenTelemetry metrics for monitoring server health
//! and performance. Metrics are exported to configured backends (Prometheus,
//! Jaeger, etc.) via the OpenTelemetry collector.
//!
//! # Metrics Collected
//!
//! - **connections_active**: Current number of active WebSocket connections (gauge)
//! - **connections_total**: Total connections since startup (counter)
//! - **requests_total**: Total JSON-RPC requests processed (counter)
//! - **request_duration**: Request processing latency distribution (histogram)
//! - **batch_size**: Batch request size distribution (histogram)
//! - **subscribers_total**: Current number of active subscriptions (gauge)
//! - **publish_total**: Total messages published (counter)
//! - **errors_total**: Total errors encountered (counter)
//!
//! # Usage
//!
//! Metrics are automatically recorded when observability is enabled via
//! `ServerBuilder::with_observability()`. They're exported periodically
//! to the configured OTLP endpoint.
//!
//! # Examples
//!
//! ```rust,no_run
//! use jrow_server::ServerMetrics;
//!
//! let metrics = ServerMetrics::new("my-service");
//!
//! // Record a connection
//! metrics.record_connection(5); // 5 active connections
//!
//! // Record a request
//! metrics.record_request("my.method", "success", 0.025);
//! ```

use opentelemetry::{
    global,
    metrics::{Counter, Gauge, Histogram, Meter},
    KeyValue,
};

/// Server metrics for monitoring
///
/// Provides OpenTelemetry metrics instruments for recording server activity.
/// All metrics are prefixed with `jrow.server.*` for easy filtering.
pub struct ServerMetrics {
    /// Number of active connections
    pub connections_active: Gauge<i64>,
    /// Total number of connections (cumulative)
    pub connections_total: Counter<u64>,
    /// Total number of requests processed
    pub requests_total: Counter<u64>,
    /// Request processing duration in seconds
    pub request_duration: Histogram<f64>,
    /// Batch size distribution
    pub batch_size: Histogram<u64>,
    /// Total number of subscribers across all topics
    pub subscribers_total: Gauge<i64>,
    /// Total number of messages published
    pub publish_total: Counter<u64>,
    /// Total number of errors
    pub errors_total: Counter<u64>,
}

impl ServerMetrics {
    /// Create a new ServerMetrics instance
    pub fn new(service_name: impl Into<String>) -> Self {
        let name: &'static str = Box::leak(service_name.into().into_boxed_str());
        let meter = global::meter(name);
        Self::new_with_meter(&meter)
    }

    /// Create a new ServerMetrics instance with a custom meter
    pub fn new_with_meter(meter: &Meter) -> Self {
        Self {
            connections_active: meter
                .i64_gauge("jrow.server.connections.active")
                .with_description("Number of active WebSocket connections")
                .build(),
            connections_total: meter
                .u64_counter("jrow.server.connections.total")
                .with_description("Total number of connections established")
                .build(),
            requests_total: meter
                .u64_counter("jrow.server.requests.total")
                .with_description("Total number of requests processed")
                .build(),
            request_duration: meter
                .f64_histogram("jrow.server.request.duration")
                .with_description("Request processing duration in seconds")
                .build(),
            batch_size: meter
                .u64_histogram("jrow.server.batch.size")
                .with_description("Number of requests in batch operations")
                .build(),
            subscribers_total: meter
                .i64_gauge("jrow.server.subscribers.total")
                .with_description("Total number of active subscribers")
                .build(),
            publish_total: meter
                .u64_counter("jrow.server.publish.total")
                .with_description("Total number of messages published")
                .build(),
            errors_total: meter
                .u64_counter("jrow.server.errors.total")
                .with_description("Total number of errors encountered")
                .build(),
        }
    }

    /// Record a new connection
    pub fn record_connection(&self, active: i64) {
        self.connections_active.record(active, &[]);
        self.connections_total.add(1, &[]);
    }

    /// Record a disconnection
    pub fn record_disconnection(&self, active: i64) {
        self.connections_active.record(active, &[]);
    }

    /// Record a request
    pub fn record_request(&self, method: &str, status: &str, duration_secs: f64) {
        let attributes = &[
            KeyValue::new("method", method.to_string()),
            KeyValue::new("status", status.to_string()),
        ];
        self.requests_total.add(1, attributes);
        self.request_duration.record(duration_secs, attributes);
    }

    /// Record a batch operation
    pub fn record_batch(&self, size: u64, mode: &str) {
        let attributes = &[KeyValue::new("mode", mode.to_string())];
        self.batch_size.record(size, attributes);
    }

    /// Update subscriber count
    pub fn update_subscribers(&self, topic: &str, count: i64) {
        let attributes = &[KeyValue::new("topic", topic.to_string())];
        self.subscribers_total.record(count, attributes);
    }

    /// Record a published message
    pub fn record_publish(&self, topic: &str) {
        let attributes = &[KeyValue::new("topic", topic.to_string())];
        self.publish_total.add(1, attributes);
    }

    /// Record an error
    pub fn record_error(&self, error_type: &str) {
        let attributes = &[KeyValue::new("error_type", error_type.to_string())];
        self.errors_total.add(1, attributes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = ServerMetrics::new("test-server");
        
        // Just test that metrics can be created without panicking
        metrics.record_connection(1);
        metrics.record_request("test_method", "success", 0.1);
        metrics.record_batch(10, "parallel");
        metrics.update_subscribers("test_topic", 5);
        metrics.record_publish("test_topic");
        metrics.record_error("test_error");
        metrics.record_disconnection(0);
    }

    #[test]
    fn test_connection_metrics() {
        let metrics = ServerMetrics::new("test-server-conn");
        
        // Record connections
        metrics.record_connection(1);
        metrics.record_connection(2);
        metrics.record_connection(3);
        
        // Record disconnections
        metrics.record_disconnection(2);
        metrics.record_disconnection(1);
        metrics.record_disconnection(0);
    }

    #[test]
    fn test_request_metrics() {
        let metrics = ServerMetrics::new("test-server-req");
        
        // Record successful requests
        metrics.record_request("add", "success", 0.05);
        metrics.record_request("multiply", "success", 0.03);
        
        // Record failed requests
        metrics.record_request("divide", "error", 0.01);
        
        // Record errors
        metrics.record_error("invalid_params");
        metrics.record_error("method_not_found");
    }

    #[test]
    fn test_batch_metrics() {
        let metrics = ServerMetrics::new("test-server-batch");
        
        // Record different batch sizes and modes
        metrics.record_batch(5, "parallel");
        metrics.record_batch(10, "sequential");
        metrics.record_batch(1, "parallel");
        metrics.record_batch(100, "parallel");
    }

    #[test]
    fn test_pubsub_metrics() {
        let metrics = ServerMetrics::new("test-server-pubsub");
        
        // Update subscriber counts
        metrics.update_subscribers("events", 5);
        metrics.update_subscribers("logs", 3);
        metrics.update_subscribers("events", 7);
        
        // Record publishes
        metrics.record_publish("events");
        metrics.record_publish("logs");
        metrics.record_publish("events");
        metrics.record_publish("events");
    }
}

