//! Client metrics definitions
//!
//! This module defines OpenTelemetry metrics for monitoring client health
//! and performance. Metrics are exported to the configured observability
//! backend (Prometheus, Jaeger, etc.).
//!
//! # Metrics Collected
//!
//! - **connection_state**: Current connection status (gauge)
//! - **requests_total**: Total requests sent (counter)
//! - **request_duration**: Request latency distribution (histogram)
//! - **errors_total**: Total errors encountered (counter)
//! - **reconnection_attempts**: Reconnection attempt count (counter)
//! - **reconnection_success**: Successful reconnections (counter)
//! - **batch_size**: Batch request size distribution (histogram)
//! - **notifications_received**: Notifications received (counter)
//!
//! # Usage
//!
//! Metrics are automatically recorded when observability is enabled via
//! `ClientBuilder::with_observability()`.
//!
//! # Examples
//!
//! ```rust,no_run
//! use jrow_client::ClientMetrics;
//!
//! let metrics = ClientMetrics::new("my-client");
//!
//! // Metrics are recorded automatically by the client
//! // They're exported periodically to the OTLP endpoint
//! ```

use opentelemetry::{
    global,
    metrics::{Counter, Gauge, Histogram, Meter},
    KeyValue,
};

/// Client metrics for monitoring
pub struct ClientMetrics {
    /// Connection state (0=disconnected, 1=connecting, 2=connected, 3=reconnecting, 4=failed)
    pub connection_state: Gauge<i64>,
    /// Total number of requests sent
    pub requests_total: Counter<u64>,
    /// Request duration in seconds
    pub request_duration: Histogram<f64>,
    /// Total number of errors
    pub errors_total: Counter<u64>,
    /// Total number of reconnection attempts
    pub reconnection_attempts: Counter<u64>,
    /// Total number of successful reconnections
    pub reconnection_success: Counter<u64>,
    /// Batch size distribution
    pub batch_size: Histogram<u64>,
    /// Total number of notifications received
    pub notifications_received: Counter<u64>,
}

impl ClientMetrics {
    /// Create a new ClientMetrics instance
    pub fn new(service_name: impl Into<String>) -> Self {
        let name: &'static str = Box::leak(service_name.into().into_boxed_str());
        let meter = global::meter(name);
        Self::new_with_meter(&meter)
    }

    /// Create a new ClientMetrics instance with a custom meter
    pub fn new_with_meter(meter: &Meter) -> Self {
        Self {
            connection_state: meter
                .i64_gauge("jrow.client.connection.state")
                .with_description("Connection state (0=disconnected, 1=connecting, 2=connected, 3=reconnecting, 4=failed)")
                .build(),
            requests_total: meter
                .u64_counter("jrow.client.requests.total")
                .with_description("Total number of requests sent")
                .build(),
            request_duration: meter
                .f64_histogram("jrow.client.request.duration")
                .with_description("Request duration in seconds")
                .build(),
            errors_total: meter
                .u64_counter("jrow.client.errors.total")
                .with_description("Total number of errors encountered")
                .build(),
            reconnection_attempts: meter
                .u64_counter("jrow.client.reconnection.attempts")
                .with_description("Total number of reconnection attempts")
                .build(),
            reconnection_success: meter
                .u64_counter("jrow.client.reconnection.success")
                .with_description("Total number of successful reconnections")
                .build(),
            batch_size: meter
                .u64_histogram("jrow.client.batch.size")
                .with_description("Number of requests in batch operations")
                .build(),
            notifications_received: meter
                .u64_counter("jrow.client.notifications.received")
                .with_description("Total number of notifications received")
                .build(),
        }
    }

    /// Update connection state
    pub fn update_connection_state(&self, state: i64) {
        self.connection_state.record(state, &[]);
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

    /// Record an error
    pub fn record_error(&self, error_type: &str) {
        let attributes = &[KeyValue::new("error_type", error_type.to_string())];
        self.errors_total.add(1, attributes);
    }

    /// Record a reconnection attempt
    pub fn record_reconnection_attempt(&self) {
        self.reconnection_attempts.add(1, &[]);
    }

    /// Record a successful reconnection
    pub fn record_reconnection_success(&self) {
        self.reconnection_success.add(1, &[]);
    }

    /// Record a batch operation
    pub fn record_batch(&self, size: u64) {
        self.batch_size.record(size, &[]);
    }

    /// Record a notification received
    pub fn record_notification(&self, method: &str) {
        let attributes = &[KeyValue::new("method", method.to_string())];
        self.notifications_received.add(1, attributes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = ClientMetrics::new("test-client");
        
        // Just test that metrics can be created without panicking
        metrics.update_connection_state(2);
        metrics.record_request("test_method", "success", 0.05);
        metrics.record_error("test_error");
        metrics.record_reconnection_attempt();
        metrics.record_reconnection_success();
        metrics.record_batch(5);
        metrics.record_notification("test_notification");
    }

    #[test]
    fn test_connection_state_metrics() {
        let metrics = ClientMetrics::new("test-client-state");
        
        // Test all connection states
        metrics.update_connection_state(0); // Disconnected
        metrics.update_connection_state(1); // Connecting
        metrics.update_connection_state(2); // Connected
        metrics.update_connection_state(3); // Reconnecting
        metrics.update_connection_state(4); // Failed
    }

    #[test]
    fn test_request_metrics() {
        let metrics = ClientMetrics::new("test-client-req");
        
        // Record successful requests
        metrics.record_request("add", "success", 0.05);
        metrics.record_request("multiply", "success", 0.03);
        
        // Record failed requests
        metrics.record_request("divide", "error", 0.01);
        metrics.record_error("timeout");
    }

    #[test]
    fn test_reconnection_metrics() {
        let metrics = ClientMetrics::new("test-client-reconnect");
        
        // Simulate reconnection attempts
        metrics.record_reconnection_attempt();
        metrics.record_reconnection_attempt();
        metrics.record_reconnection_success();
        
        metrics.record_reconnection_attempt();
        metrics.record_reconnection_attempt();
        metrics.record_reconnection_attempt();
        metrics.record_reconnection_success();
    }

    #[test]
    fn test_batch_and_notification_metrics() {
        let metrics = ClientMetrics::new("test-client-batch");
        
        // Record batch operations
        metrics.record_batch(5);
        metrics.record_batch(10);
        metrics.record_batch(1);
        
        // Record notifications
        metrics.record_notification("user.created");
        metrics.record_notification("user.updated");
        metrics.record_notification("user.deleted");
    }
}

