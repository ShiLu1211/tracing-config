//! Formatter implementations.
//!
//! Three formatter engines are available:
//!
//! | Engine      | `type` value  | Description                                 |
//! |-------------|--------------|---------------------------------------------|
//! | **default** | `"default"`  | Delegates to `tracing-subscriber` built-in  |
//! | **logback** | `"logback"`  | Logback conversion word pattern engine      |
//! | **log4j**   | `"log4j"`    | log4j PatternLayout pattern engine          |
//!
//! # Example
//!
//! ```rust
//! use tracing_config::formatter::build_formatter;
//! use tracing_config::config::FormatterConfig;
//!
//! let config = FormatterConfig {
//!     typ: "logback".to_string(),
//!     pattern: Some("%d [%thread] %-5level %logger{36} - %msg%n".to_string()),
//!     ..Default::default()
//! };
//! let formatter = build_formatter(&config).expect("failed to build formatter");
//! ```

/// Log4j pattern formatter engine.
pub mod log4j;
/// Logback pattern formatter engine.
pub mod logback;

pub use log4j::{Keyword as Log4jKeyword, Log4jFormatter, Token as Log4jToken};
pub use logback::{scan, Keyword, LogbackFormatter, Token};

use std::fmt;
use tracing::Subscriber;
use tracing_subscriber::fmt::format::Format;
use tracing_subscriber::fmt::format::Full;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::SystemTime;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::registry::LookupSpan;

use crate::config::FormatterConfig;
use crate::error::ConfigError;

/// Enum holding the built-in default formatter or one of the
/// pattern-based engines (logback, log4j).
///
/// All variants implement `FormatEvent<S, N>` generically, so the
/// enum can be passed directly to `fmt::Layer::event_format()`
/// regardless of the underlying subscriber type.
pub enum FormatterKind {
    /// Delegates to `tracing-subscriber`'s built-in formatter.
    Default(Format<Full, SystemTime>),
    /// Logback conversion word pattern engine.
    Logback(LogbackFormatter),
    /// Log4j PatternLayout pattern engine.
    Log4j(Log4jFormatter),
}

impl<S, N> FormatEvent<S, N> for FormatterKind
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        match self {
            FormatterKind::Default(f) => f.format_event(ctx, writer, event),
            FormatterKind::Logback(f) => f.format_event(ctx, writer, event),
            FormatterKind::Log4j(f) => f.format_event(ctx, writer, event),
        }
    }
}

/// Build a formatter from the given configuration.
///
/// Returns a [`FormatterKind`] which implements `FormatEvent<S, N>` generically
/// and can be used directly with `fmt::Layer::event_format()`.
///
/// # Example
///
/// ```rust
/// use tracing_config::formatter::build_formatter;
/// use tracing_config::config::FormatterConfig;
///
/// let config = FormatterConfig {
///     typ: "log4j".to_string(),
///     pattern: Some("%d [%t] %-5p %c{1.} - %m%n".to_string()),
///     ..Default::default()
/// };
/// let formatter = build_formatter(&config).expect("failed to build formatter");
/// ```
pub fn build_formatter(config: &FormatterConfig) -> Result<FormatterKind, ConfigError> {
    match config.typ.as_str() {
        "default" => Ok(FormatterKind::Default(tracing_subscriber::fmt::format())),
        "logback" => {
            let pattern = config.pattern.as_ref().ok_or_else(|| {
                ConfigError::InvalidConfig("logback formatter requires 'pattern'".into())
            })?;
            let tokens = scan(pattern).map_err(|e| ConfigError::InvalidConfig(e.to_string()))?;
            Ok(FormatterKind::Logback(LogbackFormatter::new(tokens)))
        }
        "pattern" => {
            let pattern = config.pattern.as_ref().ok_or_else(|| {
                ConfigError::InvalidConfig("pattern formatter requires 'pattern'".into())
            })?;
            let tokens = scan(pattern).map_err(|e| ConfigError::InvalidConfig(e.to_string()))?;
            Ok(FormatterKind::Logback(LogbackFormatter::new(tokens)))
        }
        "log4j" => {
            let pattern = config.pattern.as_ref().ok_or_else(|| {
                ConfigError::InvalidConfig("log4j formatter requires 'pattern'".into())
            })?;
            let tokens =
                log4j::scan(pattern).map_err(|e| ConfigError::InvalidConfig(e.to_string()))?;
            Ok(FormatterKind::Log4j(Log4jFormatter::new(tokens)))
        }
        _ => Err(ConfigError::UnknownFormatterType {
            typ: config.typ.clone(),
        }),
    }
}
