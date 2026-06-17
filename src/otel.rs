//! OpenTelemetry integration (feature-gated).
//!
//! When the `opentelemetry` feature is enabled, this module provides
//! [`build_otel_layer`] which creates an `OpenTelemetryLayer` from the
//! `[opentelemetry]` section of `tracing.toml` and installs it alongside
//! the normal fmt layer(s).
//!
//! # Example TOML
//!
//! ```toml
//! [opentelemetry]
//! enabled = true
//! endpoint = "http://localhost:4318/v1/traces"
//! service_name = "my-service"
//! service_version = "1.0.0"
//! ```
//!
//! # Example usage
//!
//! ```ignore
//! // This is called automatically by init() when the opentelemetry
//! // feature is enabled and [opentelemetry] enabled = true in the config.
//! // You should not need to call build_otel_layer() directly.
//! ```

use crate::config::OpentelemetryConfig;
use crate::error::ConfigError;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;

/// Return type holds the constructed layer **and** the tracer provider
/// that must be kept alive for the lifetime of the application.
///
/// Dropping the provider will shut down the exporter pipeline, so callers
/// should typically leak or store the `SdkTracerProvider`.
pub struct OtelGuard {
    /// The `OpenTelemetryLayer` ready to be stacked onto the subscriber.
    pub layer: tracing_opentelemetry::OpenTelemetryLayer<
        tracing_subscriber::Registry,
        opentelemetry_sdk::trace::SdkTracer,
    >,
    /// The provider that backs the tracer. Must not be dropped while
    /// the application is running.
    pub provider: SdkTracerProvider,
}

/// Build an OpenTelemetry layer from the given configuration.
///
/// Uses the HTTP/protobuf OTLP exporter (blocking reqwest), which works
/// without a Tokio runtime. The endpoint defaults to
/// `http://localhost:4318/v1/traces` if not specified.
pub fn build_otel_layer(config: &OpentelemetryConfig) -> Result<OtelGuard, ConfigError> {
    let endpoint = if config.endpoint.is_empty() {
        "http://localhost:4318/v1/traces".to_string()
    } else {
        let ep = config.endpoint.trim_end_matches('/').to_string();
        if ep.ends_with("/v1/traces") {
            ep
        } else {
            format!("{}/v1/traces", ep)
        }
    };

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(&endpoint)
        .build()
        .map_err(|e| ConfigError::OpenTelemetry(format!("failed to build OTLP exporter: {}", e)))?;

    let mut resource = Resource::builder();
    if !config.service_name.is_empty() {
        resource = resource.with_service_name(config.service_name.clone());
    }
    if !config.service_version.is_empty() {
        resource = resource.with_attribute(opentelemetry::KeyValue::new(
            "service.version",
            config.service_version.clone(),
        ));
    }
    let resource = resource.build();

    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    let service_name = if config.service_name.is_empty() {
        "tracing-config".to_string()
    } else {
        config.service_name.clone()
    };

    let tracer = provider.tracer(service_name);

    let layer = tracing_opentelemetry::layer().with_tracer(tracer);

    Ok(OtelGuard { layer, provider })
}
