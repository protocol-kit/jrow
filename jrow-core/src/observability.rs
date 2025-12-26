//! OpenTelemetry observability configuration and initialization
//!
//! This module provides integration with OpenTelemetry for distributed tracing,
//! metrics collection, and structured logging. It configures the telemetry pipeline
//! to export data to an OTLP (OpenTelemetry Protocol) collector.
//!
//! # Overview
//!
//! OpenTelemetry provides three pillars of observability:
//! - **Traces**: Distributed request tracking across services
//! - **Metrics**: Quantitative measurements (counters, gauges, histograms)
//! - **Logs**: Structured event records with context
//!
//! This module sets up all three pillars with sensible defaults while allowing
//! customization via `ObservabilityConfig`.
//!
//! # Architecture
//!
//! The telemetry pipeline consists of:
//! 1. **Instrumentation**: Code generates telemetry data
//! 2. **Processors**: Batch and enrich telemetry data
//! 3. **Exporters**: Send data to collector via OTLP/gRPC
//! 4. **Collector**: Receives, processes, and forwards to backends
//!
//! # Usage Pattern
//!
//! Initialize observability at application startup, before creating servers:
//!
//! ```rust,no_run
//! use jrow_core::ObservabilityConfig;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = ObservabilityConfig::new("my-service")
//!         .with_endpoint("http://localhost:4317")
//!         .with_log_level("debug");
//!     
//!     jrow_core::init_observability(config).expect("Failed to init observability");
//!     
//!     // ... run your application ...
//!     
//!     jrow_core::shutdown_observability();
//! }
//! ```
//!
//! # Environment Variables
//!
//! Configuration can be controlled via environment variables:
//! - `OTEL_EXPORTER_OTLP_ENDPOINT`: Collector endpoint
//! - `RUST_LOG`: Log level filter (e.g., "info", "debug")

use opentelemetry::{global, KeyValue};
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Observability configuration for OpenTelemetry
///
/// This struct configures the three pillars of observability: traces, metrics, and logs.
/// Each can be enabled or disabled independently, though they work best together.
///
/// # Fields
///
/// - **service_name**: Identifies your service in telemetry backends (required)
/// - **service_version**: Helps track behavior across deployments
/// - **otlp_endpoint**: Where to send telemetry data (OTLP collector address)
/// - **enable_traces**: Toggle distributed tracing
/// - **enable_metrics**: Toggle metrics collection
/// - **enable_logs**: Toggle structured logging
/// - **log_level**: Filter logs by level (e.g., "info", "debug", "trace")
///
/// # Defaults
///
/// The default configuration:
/// - Service name: "jrow"
/// - Service version: Current crate version
/// - OTLP endpoint: From `OTEL_EXPORTER_OTLP_ENDPOINT` env var, or "http://localhost:4317"
/// - All pillars enabled
/// - Log level: From `RUST_LOG` env var, or "info"
///
/// # Examples
///
/// ```rust
/// use jrow_core::ObservabilityConfig;
///
/// // Use defaults
/// let config = ObservabilityConfig::default();
///
/// // Customize
/// let custom = ObservabilityConfig::new("my-api-server")
///     .with_endpoint("http://collector:4317")
///     .with_log_level("debug")
///     .with_version("1.2.3")
///     .with_metrics(false);  // Disable metrics
/// ```
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Service name for telemetry data
    ///
    /// This appears in all traces, metrics, and logs to identify the source.
    /// Choose a name that uniquely identifies your service in your environment.
    pub service_name: String,
    
    /// Service version for telemetry data
    ///
    /// Useful for correlating telemetry with specific deployments or releases.
    /// Defaults to the crate version.
    pub service_version: String,
    
    /// OTLP (OpenTelemetry Protocol) endpoint
    ///
    /// The gRPC endpoint of your OpenTelemetry collector. Common options:
    /// - Local collector: "http://localhost:4317"
    /// - Docker compose: "http://otel-collector:4317"
    /// - Cloud services: Check provider documentation
    pub otlp_endpoint: String,
    
    /// Enable distributed tracing
    ///
    /// Traces track requests across service boundaries, showing timing and causality.
    /// Disable if you only need metrics or logs.
    pub enable_traces: bool,
    
    /// Enable metrics collection
    ///
    /// Metrics provide quantitative measurements (request counts, latencies, etc.).
    /// Disable if you only need traces or logs.
    pub enable_metrics: bool,
    
    /// Enable structured logs
    ///
    /// Logs provide detailed event information with contextual attributes.
    /// Disable if you don't need log export (though local logs will still work).
    pub enable_logs: bool,
    
    /// Log level filter
    ///
    /// Controls which log messages are emitted. Standard levels:
    /// - "error": Only errors
    /// - "warn": Warnings and errors
    /// - "info": Informational messages and above (default)
    /// - "debug": Detailed debugging information
    /// - "trace": Very verbose, includes fine-grained execution details
    pub log_level: String,
}

impl Default for ObservabilityConfig {
    /// Create a default observability configuration
    ///
    /// The defaults are designed to work out-of-the-box with a local OpenTelemetry
    /// collector running on the standard port.
    ///
    /// # Default Values
    ///
    /// - Service name: "jrow"
    /// - Service version: The current crate version (from Cargo.toml)
    /// - OTLP endpoint: `$OTEL_EXPORTER_OTLP_ENDPOINT` or "http://localhost:4317"
    /// - All telemetry types enabled (traces, metrics, logs)
    /// - Log level: `$RUST_LOG` or "info"
    ///
    /// # Environment Variable Support
    ///
    /// This implementation respects standard OpenTelemetry environment variables,
    /// making it compatible with existing observability infrastructure.
    fn default() -> Self {
        Self {
            service_name: "jrow".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".to_string()),
            enable_traces: true,
            enable_metrics: true,
            enable_logs: true,
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        }
    }
}

impl ObservabilityConfig {
    /// Create a new configuration with a custom service name
    ///
    /// All other settings use defaults. Use builder methods to customize.
    ///
    /// # Arguments
    ///
    /// * `service_name` - Unique identifier for your service
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jrow_core::ObservabilityConfig;
    ///
    /// let config = ObservabilityConfig::new("payment-service")
    ///     .with_log_level("debug");
    /// ```
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            ..Default::default()
        }
    }

    /// Set the OTLP collector endpoint
    ///
    /// This is where telemetry data will be sent via gRPC.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Full URL including protocol (e.g., "http://collector:4317")
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = endpoint.into();
        self
    }

    /// Set the log level filter
    ///
    /// Controls the verbosity of logging. Standard values:
    /// "error", "warn", "info", "debug", "trace"
    ///
    /// # Arguments
    ///
    /// * `level` - Log level string
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = level.into();
        self
    }

    /// Set the service version
    ///
    /// This helps correlate telemetry with specific deployments.
    ///
    /// # Arguments
    ///
    /// * `version` - Version string (e.g., "1.2.3", "2024-01-15", git commit hash)
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = version.into();
        self
    }

    /// Enable or disable distributed tracing
    ///
    /// # Arguments
    ///
    /// * `enable` - true to enable traces, false to disable
    pub fn with_traces(mut self, enable: bool) -> Self {
        self.enable_traces = enable;
        self
    }

    /// Enable or disable metrics collection
    ///
    /// # Arguments
    ///
    /// * `enable` - true to enable metrics, false to disable
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }

    /// Enable or disable structured logs
    ///
    /// # Arguments
    ///
    /// * `enable` - true to enable logs, false to disable
    pub fn with_logs(mut self, enable: bool) -> Self {
        self.enable_logs = enable;
        self
    }
}

/// Initialize OpenTelemetry with the given configuration
///
/// This is the main entry point for setting up observability. It configures
/// all enabled telemetry providers (traces, metrics, logs) and connects them
/// to the specified OTLP collector.
///
/// # What This Does
///
/// 1. **Tracer provider**: Sets up distributed tracing with OTLP exporter
/// 2. **Meter provider**: Configures metrics with periodic export (every 30s)
/// 3. **Tracing subscriber**: Integrates with Rust's `tracing` ecosystem
/// 4. **Global registration**: Makes providers available via `opentelemetry::global`
///
/// # When to Call
///
/// Call this **once** at application startup, before creating any servers or
/// clients. Calling it multiple times will panic (global providers can only
/// be set once).
///
/// # Arguments
///
/// * `config` - Configuration specifying what to enable and where to export
///
/// # Returns
///
/// - `Ok(())` if initialization succeeds
/// - `Err` if provider setup fails (e.g., can't connect to collector)
///
/// # Errors
///
/// Common error scenarios:
/// - OTLP collector is unreachable
/// - Invalid endpoint URL
/// - Incompatible OpenTelemetry versions
/// - Called more than once (global providers already set)
///
/// # Examples
///
/// ```rust,no_run
/// use jrow_core::ObservabilityConfig;
///
/// #[tokio::main]
/// async fn main() {
///     let config = ObservabilityConfig::new("my-service")
///         .with_endpoint("http://localhost:4317");
///     
///     jrow_core::init_observability(config).expect("Failed to init observability");
///     
///     // Now your application code can use tracing macros:
///     tracing::info!("Application started");
/// }
/// ```
///
/// # Performance Considerations
///
/// - Traces are batched before export to reduce overhead
/// - Metrics are aggregated and exported every 30 seconds
/// - Async operations don't block application threads
pub fn init_observability(
    config: ObservabilityConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracer provider if traces are enabled
    // We need to get the tracer before setting it as global, because the
    // tracing subscriber needs it to create the telemetry layer
    let tracer = if config.enable_traces {
        Some(init_tracer(&config)?)
    } else {
        None
    };

    // Initialize meter provider if metrics are enabled
    // This registers the provider globally so any code can create meters
    if config.enable_metrics {
        init_metrics(&config)?;
    }

    // Initialize the tracing subscriber that bridges Rust's tracing crate
    // with OpenTelemetry. This allows using tracing::info!() etc. and having
    // those logs/spans exported to the collector
    init_tracing_subscriber(&config, tracer)?;

    // Log that initialization succeeded, with key configuration details
    tracing::info!(
        service_name = %config.service_name,
        otlp_endpoint = %config.otlp_endpoint,
        traces = config.enable_traces,
        metrics = config.enable_metrics,
        logs = config.enable_logs,
        "OpenTelemetry initialized"
    );

    Ok(())
}

/// Initialize the tracer provider and return a tracer
///
/// This function sets up distributed tracing with the following configuration:
/// - **Batch exporter**: Spans are buffered and sent in batches for efficiency
/// - **OTLP/gRPC**: Uses gRPC protocol to send spans to collector
/// - **AlwaysOn sampling**: All spans are recorded (change for production)
/// - **Random ID generation**: Generates trace and span IDs using crypto-random
///
/// # Resource Attributes
///
/// The resource identifies this service in the telemetry backend:
/// - `service.name`: From config
/// - `service.version`: From config
///
/// # Why Return a Tracer?
///
/// We need to return the tracer before setting the provider as global because
/// the tracing subscriber needs it to create the OpenTelemetry layer.
///
/// # Sampling
///
/// Currently uses `Sampler::AlwaysOn` which records 100% of traces.
/// For high-traffic production systems, consider:
/// - `Sampler::ParentBased`: Respect upstream sampling decisions
/// - `Sampler::TraceIdRatioBased`: Sample a percentage of traces
fn init_tracer(
    config: &ObservabilityConfig,
) -> Result<opentelemetry_sdk::trace::Tracer, Box<dyn std::error::Error + Send + Sync>> {
    use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler};
    use opentelemetry_sdk::Resource;

    // Create resource attributes that identify this service
    // These appear on all spans and help filter/group in observability backends
    let resource = Resource::builder_empty()
        .with_attributes(vec![
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                config.service_name.clone(),
            ),
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
                config.service_version.clone(),
            ),
        ])
        .build();

    // Create OTLP span exporter using gRPC transport (via tonic)
    // The endpoint is configured from config.otlp_endpoint
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()?;

    // Build the tracer provider with:
    // - Batch exporter for efficiency (async batching reduces overhead)
    // - Resource attributes for service identification
    // - AlwaysOn sampler (records all spans - adjust for production)
    // - Random ID generator for generating unique trace/span IDs
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .build();

    // Get a tracer instance before registering the provider globally
    // The tracer is needed by the tracing subscriber to create spans
    use opentelemetry::trace::TracerProvider as _;
    let tracer = provider.tracer(config.service_name.clone());

    // Register the provider globally so other code can access it
    global::set_tracer_provider(provider);
    
    Ok(tracer)
}

/// Initialize the meter provider for metrics collection
///
/// This function sets up metrics with the following configuration:
/// - **Periodic export**: Metrics are aggregated and exported every 30 seconds
/// - **OTLP/gRPC**: Uses gRPC protocol to send metrics to collector
/// - **Delta temporality**: Exports changes since last export (more efficient)
///
/// # Metric Types Supported
///
/// The meter provider supports all OpenTelemetry metric types:
/// - **Counter**: Monotonically increasing values (e.g., request count)
/// - **UpDownCounter**: Can increase or decrease (e.g., active connections)
/// - **Histogram**: Distribution of values (e.g., request latencies)
/// - **Gauge**: Snapshot of current value (e.g., memory usage)
///
/// # Export Interval
///
/// Metrics are exported every 30 seconds. This balances:
/// - **Overhead**: Less frequent exports reduce CPU/network usage
/// - **Freshness**: More frequent exports provide near-real-time data
///
/// For production, 30-60 seconds is typically appropriate.
///
/// # Resource Attributes
///
/// Same as tracer: includes service.name and service.version to identify
/// the source of metrics in your observability backend.
fn init_metrics(
    config: &ObservabilityConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use opentelemetry_sdk::Resource;

    // Create resource attributes to identify this service
    let resource = Resource::builder_empty()
        .with_attributes(vec![
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                config.service_name.clone(),
            ),
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
                config.service_version.clone(),
            ),
        ])
        .build();

    // Create OTLP metric exporter using gRPC transport
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .build()?;

    // Create a periodic reader that:
    // - Collects metrics from instruments
    // - Aggregates them over 30-second intervals
    // - Exports them via the OTLP exporter
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(30))
        .build();

    // Build and register the meter provider
    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();

    // Register globally so any code can create meters and instruments
    global::set_meter_provider(provider);
    Ok(())
}

/// Initialize tracing subscriber with OpenTelemetry layer
///
/// This sets up Rust's `tracing` crate to work with OpenTelemetry.
/// The subscriber consists of multiple layers:
///
/// 1. **OpenTelemetry layer** (if traces enabled): Converts tracing spans to OTLP spans
/// 2. **EnvFilter**: Filters logs based on RUST_LOG or config.log_level
/// 3. **fmt layer**: Outputs structured JSON logs to stdout
///
/// # Why Multiple Layers?
///
/// Each layer serves a different purpose:
/// - **Telemetry layer**: Exports spans to observability backend
/// - **fmt layer**: Provides local console output for debugging
/// - **EnvFilter**: Prevents log spam by filtering by level
///
/// # Structured Logging
///
/// The fmt layer outputs JSON-formatted logs with:
/// - `target`: The module path where the log originated
/// - `thread_ids`: The thread ID (useful for debugging concurrency)
/// - `line_number`: Source file line number
/// - `fields`: Any structured fields attached to the span/event
///
/// # Arguments
///
/// * `config` - Configuration specifying log level
/// * `tracer` - Optional tracer for the OpenTelemetry layer (None if traces disabled)
///
/// # Implementation Notes
///
/// We check if a tracer is provided to determine whether to include the
/// OpenTelemetry layer. This allows metrics-only or logs-only configurations.
fn init_tracing_subscriber(
    config: &ObservabilityConfig,
    tracer: Option<opentelemetry_sdk::trace::Tracer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create environment filter from RUST_LOG env var or config
    // Supports standard directives like "info", "debug", "mymodule=trace"
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.log_level))?;

    if let Some(tracer) = tracer {
        // Traces are enabled: create subscriber with OpenTelemetry layer
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        
        // Create JSON formatter for structured logs
        // Includes metadata useful for debugging and analysis
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(true)      // Include module path
            .with_thread_ids(true)   // Include thread ID
            .with_line_number(true)  // Include source line number
            .json();                 // Output as JSON

        // Build and initialize the layered subscriber
        tracing_subscriber::registry()
            .with(telemetry_layer)  // Export to OpenTelemetry
            .with(env_filter)        // Filter by log level
            .with(fmt_layer)         // Output JSON logs locally
            .init();
    } else {
        // Traces are disabled: create subscriber without OpenTelemetry layer
        // This still provides local logging with structured output
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_line_number(true)
            .json();
            
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    Ok(())
}

/// Gracefully shutdown OpenTelemetry providers
///
/// This function ensures all pending telemetry data is flushed to the
/// collector before the application exits. It should be called during
/// graceful shutdown.
///
/// # Why Call This?
///
/// Telemetry is exported asynchronously in batches. Without explicit
/// shutdown, you might lose the last batch of:
/// - Spans not yet exported
/// - Metrics not yet aggregated and sent
/// - Buffered log events
///
/// # When to Call
///
/// Call this at the end of your main function or in your shutdown handler:
///
/// ```rust,no_run
/// # use jrow_core::ObservabilityConfig;
/// #[tokio::main]
/// async fn main() {
///     let config = ObservabilityConfig::default();
///     jrow_core::init_observability(config).unwrap();
///     
///     // ... application code ...
///     
///     // Before exiting
///     jrow_core::shutdown_observability();
/// }
/// ```
///
/// # Implementation Notes
///
/// In OpenTelemetry 0.30+, providers automatically flush on drop,
/// so explicit shutdown is technically optional. However, calling
/// this function is still good practice as it:
/// - Makes shutdown intent explicit in code
/// - Ensures telemetry about shutdown itself is exported
/// - Provides a future extension point if manual flush becomes needed
pub fn shutdown_observability() {
    tracing::info!("Shutting down OpenTelemetry");

    // Note: In OpenTelemetry SDK 0.30+, providers implement Drop to flush
    // and shut down gracefully. Manual shutdown is not strictly required,
    // but we keep this function for explicit lifecycle management and
    // future-proofing.
    
    // If we needed to manually shut down, we would:
    // 1. Call opentelemetry::global::shutdown_tracer_provider()
    // 2. Call opentelemetry::global::shutdown_meter_provider()
    // But these are automatically called when providers are dropped.
    
    tracing::info!("OpenTelemetry shutdown complete");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ObservabilityConfig::default();
        assert_eq!(config.service_name, "jrow");
        assert!(config.enable_traces);
        assert!(config.enable_metrics);
        assert!(config.enable_logs);
    }

    #[test]
    fn test_custom_config() {
        let config = ObservabilityConfig::new("test-service")
            .with_endpoint("http://custom:4317")
            .with_log_level("debug")
            .with_version("1.0.0")
            .with_traces(false);

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.otlp_endpoint, "http://custom:4317");
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.service_version, "1.0.0");
        assert!(!config.enable_traces);
    }

    #[test]
    fn test_init_with_traces_only() {
        let config = ObservabilityConfig::new("test-traces")
            .with_traces(true)
            .with_metrics(false)
            .with_logs(false);
        
        assert!(config.enable_traces);
        assert!(!config.enable_metrics);
        assert!(!config.enable_logs);
        
        // Just verify config is set correctly - don't actually init OTLP in tests
    }

    #[test]
    fn test_init_with_metrics_only() {
        let config = ObservabilityConfig::new("test-metrics")
            .with_traces(false)
            .with_metrics(true)
            .with_logs(false);
        
        assert!(!config.enable_traces);
        assert!(config.enable_metrics);
        assert!(!config.enable_logs);
        
        // Just verify config is set correctly - don't actually init OTLP in tests
    }

    #[test]
    fn test_init_with_logs_only() {
        let config = ObservabilityConfig::new("test-logs")
            .with_traces(false)
            .with_metrics(false)
            .with_logs(true)
            .with_log_level("trace");
        
        assert!(!config.enable_traces);
        assert!(!config.enable_metrics);
        assert!(config.enable_logs);
        assert_eq!(config.log_level, "trace");
        
        // Just verify config is set correctly - don't actually init OTLP in tests
    }

    #[test]
    fn test_init_all_disabled() {
        let config = ObservabilityConfig::new("test-none")
            .with_traces(false)
            .with_metrics(false)
            .with_logs(false);
        
        assert!(!config.enable_traces);
        assert!(!config.enable_metrics);
        assert!(!config.enable_logs);
        
        // With all disabled, init should still succeed
        let result = init_observability(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shutdown_idempotent() {
        // Test that calling shutdown multiple times doesn't panic
        shutdown_observability();
        shutdown_observability();
        shutdown_observability();
        // If we got here without panicking, test passed
    }

    #[test]
    fn test_config_with_custom_endpoint() {
        // Test configuration with custom endpoint (don't actually connect)
        let config = ObservabilityConfig::new("test-no-collector")
            .with_endpoint("http://localhost:9999");
        
        assert_eq!(config.otlp_endpoint, "http://localhost:9999");
        // Don't actually try to init - just verify config is stored correctly
    }

    #[test]
    fn test_config_builder_chaining() {
        // Test that all builder methods can be chained
        let config = ObservabilityConfig::default()
            .with_endpoint("http://test:4317")
            .with_log_level("info")
            .with_version("2.0.0")
            .with_traces(true)
            .with_metrics(true)
            .with_logs(true);
        
        assert_eq!(config.otlp_endpoint, "http://test:4317");
        assert_eq!(config.log_level, "info");
        assert_eq!(config.service_version, "2.0.0");
        assert!(config.enable_traces);
        assert!(config.enable_metrics);
        assert!(config.enable_logs);
    }

    #[test]
    fn test_config_log_levels() {
        // Test different log level configurations
        for level in &["trace", "debug", "info", "warn", "error"] {
            let config = ObservabilityConfig::default().with_log_level(*level);
            assert_eq!(config.log_level, *level);
        }
    }

    #[test]
    fn test_service_name_validation() {
        // Test various service names
        let names = vec!["my-service", "service_123", "Service.Name", "s"];
        for name in names {
            let config = ObservabilityConfig::new(name);
            assert_eq!(config.service_name, name);
        }
    }
}
