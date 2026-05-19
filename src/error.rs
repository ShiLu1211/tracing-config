//! Error types for tracing-config.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to parse TOML: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("pattern parse error at position {position}: {message}")]
    PatternParse { message: String, position: usize },

    #[error("unrecognized appender kind '{kind}'")]
    UnknownAppenderKind { kind: String },

    #[error("unrecognized formatter type '{typ}'")]
    UnknownFormatterType { typ: String },

    #[error("config file not found and no default")]
    NoConfig,

    #[error("rolling_file appender requires 'dir' field")]
    RollingMissingDir,
}
