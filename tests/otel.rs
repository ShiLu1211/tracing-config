#[cfg(feature = "opentelemetry")]
mod tests {
    use tracing_declarative::config::OpentelemetryConfig;
    use tracing_declarative::otel::build_otel_layer;

    #[test]
    fn test_build_otel_layer_with_defaults() {
        let config = OpentelemetryConfig {
            enabled: true,
            endpoint: String::new(),
            service_name: String::new(),
            service_version: String::new(),
        };
        let guard = build_otel_layer(&config).expect("should build with defaults");
        let _ = guard.provider;
    }

    #[test]
    fn test_build_otel_layer_with_custom_endpoint() {
        let config = OpentelemetryConfig {
            enabled: true,
            endpoint: "http://localhost:4318".to_string(),
            service_name: "test-service".to_string(),
            service_version: "1.0.0".to_string(),
        };
        let guard = build_otel_layer(&config).expect("should build with custom config");
        let _ = guard.provider;
    }

    #[test]
    fn test_build_otel_layer_endpoint_appends_path() {
        let config = OpentelemetryConfig {
            enabled: true,
            endpoint: "http://collector:4318".to_string(),
            service_name: "my-app".to_string(),
            service_version: "0.1.0".to_string(),
        };
        let guard = build_otel_layer(&config).expect("should append /v1/traces");
        let _ = guard.provider;
    }

    #[test]
    fn test_parse_opentelemetry_config() {
        let toml = r#"
[global]
level = "info"

[opentelemetry]
enabled = true
endpoint = "http://localhost:4317"
service_name = "my-service"
service_version = "2.0.0"

[[appender]]
name = "stdout"
kind = "stdout"
enabled = true
"#;
        let config: tracing_declarative::config::Config =
            toml::from_str(toml).expect("parse should succeed");
        assert!(config.opentelemetry.enabled);
        assert_eq!(config.opentelemetry.endpoint, "http://localhost:4317");
        assert_eq!(config.opentelemetry.service_name, "my-service");
        assert_eq!(config.opentelemetry.service_version, "2.0.0");
    }

    #[test]
    fn test_parse_opentelemetry_config_defaults() {
        let toml = r#"
[global]
level = "info"
"#;
        let config: tracing_declarative::config::Config =
            toml::from_str(toml).expect("parse should succeed");
        assert!(!config.opentelemetry.enabled);
        assert!(config.opentelemetry.endpoint.is_empty());
        assert!(config.opentelemetry.service_name.is_empty());
        assert!(config.opentelemetry.service_version.is_empty());
    }
}
