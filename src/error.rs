//! Error types for tracing-config.

use thiserror::Error;

/// Errors that can occur during configuration parsing or tracing initialization.
///
/// # Example
///
/// ```
/// use tracing_declarative::error::ConfigError;
///
/// let result = tracing_declarative::parse("not valid toml {{{");
/// match result {
///     Err(ConfigError::TomlParse(e)) => println!("TOML error: {e}"),
///     Err(ConfigError::InvalidConfig(msg)) => println!("invalid: {msg}"),
///     Err(ConfigError::PatternParse { message, position }) => {
///         println!("pattern error at {position}: {message}")
///     }
///     Err(e) => println!("other error: {e}"),
///     Ok(_) => println!("ok"),
/// }
/// ```
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to parse TOML content.
    #[error("failed to parse TOML: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// I/O error reading a config file or opening an appender.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Semantically invalid configuration value.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Pattern parse error at a specific character position.
    #[error("pattern parse error at position {position}: {message}")]
    PatternParse {
        /// Human-readable description of the parse error.
        message: String,
        /// Zero-based byte offset in the pattern string.
        position: usize,
    },

    /// Unrecognized appender `kind` value.
    #[error("unrecognized appender kind '{kind}'")]
    UnknownAppenderKind {
        /// The invalid kind string.
        kind: String,
    },

    /// Unrecognized formatter `type` value.
    #[error("unrecognized formatter type '{typ}'")]
    UnknownFormatterType {
        /// The invalid type string.
        typ: String,
    },

    /// Config file not found and no built-in default.
    #[error("config file not found and no default")]
    NoConfig,

    /// `rolling_file` appender missing required `dir` field.
    #[error("rolling_file appender requires 'dir' field")]
    RollingMissingDir,

    /// OpenTelemetry exporter or provider construction failed.
    #[error("OpenTelemetry error: {0}")]
    #[cfg(feature = "opentelemetry")]
    OpenTelemetry(String),
}
