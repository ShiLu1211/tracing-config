//! Tests for error types.

use tracing_config::error::ConfigError;

#[test]
fn test_invalid_config_error() {
    let err = ConfigError::InvalidConfig("test error".into());
    assert!(err.to_string().contains("test error"));
}

#[test]
fn test_unknown_appender_kind_error() {
    let err = ConfigError::UnknownAppenderKind {
        kind: "unknown".into(),
    };
    assert!(err.to_string().contains("unknown"));
}

#[test]
fn test_unknown_formatter_type_error() {
    let err = ConfigError::UnknownFormatterType {
        typ: "custom".into(),
    };
    assert!(err.to_string().contains("custom"));
}

#[test]
fn test_pattern_parse_error() {
    let err = ConfigError::PatternParse {
        message: "invalid pattern".into(),
        position: 10,
    };
    assert!(err.to_string().contains("10"));
    assert!(err.to_string().contains("invalid pattern"));
}

#[test]
fn test_no_config_error() {
    let err = ConfigError::NoConfig;
    assert!(err.to_string().contains("not found"));
}

#[test]
fn test_rolling_missing_dir_error() {
    let err = ConfigError::RollingMissingDir;
    assert!(err.to_string().contains("dir"));
}
