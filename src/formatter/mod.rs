//! Formatter implementations.

pub mod logback;

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

/// Enum holding either the built-in tracing default formatter or a logback formatter.
///
/// Both variants implement `FormatEvent<S, N>` generically, so this enum can be passed
/// directly to `fmt::Layer::event_format()` regardless of the underlying subscriber type.
pub enum FormatterKind {
    Default(Format<Full, SystemTime>),
    Logback(LogbackFormatter),
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
        }
    }
}

/// Build a formatter from the given configuration.
///
/// Returns a `FormatterKind` which implements `FormatEvent<S, N>` generically
/// and can be used directly with `fmt::Layer::event_format()`.
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
            // Backwards compatibility with "pattern" type
            let pattern = config.pattern.as_ref().ok_or_else(|| {
                ConfigError::InvalidConfig("pattern formatter requires 'pattern'".into())
            })?;
            let tokens = scan(pattern).map_err(|e| ConfigError::InvalidConfig(e.to_string()))?;
            Ok(FormatterKind::Logback(LogbackFormatter::new(tokens)))
        }
        _ => Err(ConfigError::UnknownFormatterType {
            typ: config.typ.clone(),
        }),
    }
}
