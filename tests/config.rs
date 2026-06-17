//! Tests for configuration parsing.

#[test]
fn test_sampling_config_parsing() {
    let config = r#"
[global]
level = "info"

[sampling]
enabled = true
rate_per_second = 500
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert!(parsed.sampling.enabled);
    assert_eq!(parsed.sampling.rate_per_second, 500);
}

#[test]
fn test_sampling_config_defaults() {
    let config = r#"
[global]
level = "info"
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert!(!parsed.sampling.enabled);
    assert_eq!(parsed.sampling.rate_per_second, 0);
}

#[test]
fn test_opentelemetry_config_parsing() {
    let config = r#"
[global]
level = "info"

[opentelemetry]
enabled = true
endpoint = "http://localhost:4317"
service_name = "my-service"
service_version = "1.0.0"
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert!(parsed.opentelemetry.enabled);
    assert_eq!(parsed.opentelemetry.endpoint, "http://localhost:4317");
    assert_eq!(parsed.opentelemetry.service_name, "my-service");
    assert_eq!(parsed.opentelemetry.service_version, "1.0.0");
}

#[test]
fn test_opentelemetry_config_defaults() {
    let config = r#"
[global]
level = "info"
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert!(!parsed.opentelemetry.enabled);
    assert!(parsed.opentelemetry.endpoint.is_empty());
}

#[test]
fn test_formatter_config_with_all_options() {
    let config = r#"
[global]
level = "info"

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "default"
compact = true
with_target = true
with_file = true
with_line = true
with_thread = true
with_level = true
with_time = true
time_format = "%H:%M:%S"
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    let formatter = &parsed.appenders[0].formatter;
    assert_eq!(formatter.typ, "default");
    assert!(formatter.compact);
    assert!(formatter.with_target);
    assert!(formatter.with_file);
    assert!(formatter.with_line);
    assert!(formatter.with_thread);
    assert!(formatter.with_level);
    assert!(formatter.with_time);
    assert_eq!(formatter.time_format, "%H:%M:%S");
}

#[test]
fn test_global_config_parsing() {
    let config = r#"
[global]
level = "debug"
ansi = false
span_events = "new"
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert_eq!(parsed.global.level, "debug");
    assert!(!parsed.global.ansi);
    assert_eq!(parsed.global.span_events, "new");
}

#[test]
fn test_filter_config_parsing() {
    let config = r#"
[global]
level = "info"

[filter]
default_level = "warn"
directives = ["crate1=debug", "crate2=trace"]
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert_eq!(parsed.filter.default_level, "warn");
    assert_eq!(parsed.filter.directives.len(), 2);
    assert_eq!(parsed.filter.directives[0], "crate1=debug");
}

#[test]
fn test_appender_level_filter() {
    let config = r#"
[global]
level = "info"

[[appender]]
name = "stdout"
kind = "stdout"
level = "debug"
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert_eq!(parsed.appenders[0].level.as_ref().unwrap(), "debug");
}

#[test]
fn test_appender_max_size_and_files() {
    let config = r#"
[global]
level = "info"

[[appender]]
name = "rolling"
kind = "rolling_file"
dir = "/tmp"
max_size = 10485760
max_files = 5
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert_eq!(parsed.appenders[0].max_size.unwrap(), 10485760);
    assert_eq!(parsed.appenders[0].max_files.unwrap(), 5);
}
