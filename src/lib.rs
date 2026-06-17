//! tracing-config — Declarative tracing initialization via `tracing.toml`.
//!
//! # Quick start
//!
//! Create a `tracing.toml` in your project root:
//!
//! ```toml
//! [global]
//! level = "info"
//! ansi = true
//!
//! [filter]
//! default_level = "info"
//!
//! [[appender]]
//! name = "stdout"
//! kind = "stdout"
//! enabled = true
//!
//! [appender.formatter]
//! type = "logback"
//! pattern = "%d [%thread] %-5level %logger{36} - %msg%n"
//! ```
//!
//! Then initialize in your `main`:
//!
//! ```no_run
//! tracing_config::init().expect("tracing init failed");
//! tracing::info!("application started");
//! ```
//!
//! # Configuration search path
//!
//! `init()` looks for `tracing.toml` in this order:
//! 1. `TRACING_CONFIG` environment variable
//! 2. `./tracing.toml` (current working directory)
//! 3. `./tracing.toml` (executable directory)
//!
//! If no file is found, a built-in default is used (INFO level, stdout,
//! default formatter).
//!
//! # Feature flags
//!
//! - **`hot-reload`** (default) — File-watch support (unstable; `tracing`
//!   does not support re-initialization, so this is a no-op in practice).
//! - **`opentelemetry`** — OpenTelemetry OTLP trace export via HTTP/protobuf.
//!
//! # Supported formatter engines
//!
//! | `type`      | Syntax                     | Module                      |
//! |-------------|---------------------------|------------------------------|
//! | `"default"` | tracing-subscriber built-in| `formatter::default`         |
//! | `"logback"` | logback conversion words  | `formatter::logback`         |
//! | `"log4j"`   | log4j PatternLayout       | `formatter::log4j`           |

#![deny(missing_docs)]

use std::io::Write as IoWrite;
use std::path::Path;

use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::config::{AppenderConfig, Config};
use crate::error::ConfigError;
use crate::formatter::build_formatter;
use crate::sampling::SamplingWriter;
use crate::span_fields::SpanFieldsLayer;

/// Appender module (currently empty — logic lives in `lib.rs`).
pub mod appender;
/// Configuration structures parsed from `tracing.toml`.
pub mod config;
/// Error types.
pub mod error;
/// Formatter engines (default, logback, log4j).
pub mod formatter;
/// Sampling / rate-limiting writer.
pub mod sampling;
/// Span field capture layer (`%X{key}` / `%mdc` support).
pub mod span_fields;
/// Windows console ANSI support.
pub mod windows;

/// OpenTelemetry integration (feature-gated).
#[cfg(feature = "opentelemetry")]
pub mod otel;

/// Hot-reload support (unstable — see module docs).
#[cfg(feature = "hot-reload")]
pub mod hot_reload;

// Build-time version info injected by build.rs
include!(concat!(env!("OUT_DIR"), "/version.rs"));

/// Re-export of `ReloadHandle` when `hot-reload` feature is enabled.
#[cfg(feature = "hot-reload")]
pub use hot_reload::ReloadHandle;

#[cfg(feature = "opentelemetry")]
use otel::build_otel_layer;

/// Initialize tracing from the default `tracing.toml` search path.
///
/// # Example
///
/// ```no_run
/// tracing_config::init().expect("tracing init failed");
/// tracing::info!("hello");
/// ```
///
/// # Errors
///
/// Returns an error if the config file exists but cannot be parsed,
/// or if an appender cannot be created (e.g. invalid rolling file path).
pub fn init() -> Result<(), ConfigError> {
    let config = Config::from_default_file()?;
    init_with_config(config)
}

/// Initialize tracing from a specific file path.
///
/// # Example
///
/// ```no_run
/// tracing_config::init_from_file("/etc/myapp/tracing.toml")
///     .expect("tracing init failed");
/// ```
pub fn init_from_file(path: impl AsRef<Path>) -> Result<(), ConfigError> {
    let config = Config::from_file(path)?;
    init_with_config(config)
}

/// Initialize tracing from a TOML string.
///
/// Useful for embedded configs or tests.
///
/// # Example
///
/// ```
/// let config = r#"
/// [global]
/// level = "debug"
///
/// [[appender]]
/// name = "stdout"
/// kind = "stdout"
/// enabled = true
///
/// [appender.formatter]
/// type = "default"
/// "#;
/// tracing_config::init_from_str(config).expect("tracing init failed");
/// ```
pub fn init_from_str(content: &str) -> Result<(), ConfigError> {
    let config: Config = toml::from_str(content)?;
    init_with_config(config)
}

/// Parse configuration without initializing tracing.
///
/// Useful for validating a config at startup or inspecting the parsed
/// structure.
///
/// # Example
///
/// ```
/// let config = r#"
/// [global]
/// level = "info"
///
/// [[appender]]
/// name = "stdout"
/// kind = "stdout"
/// enabled = true
/// "#;
/// let parsed = tracing_config::parse(config).expect("parse failed");
/// assert_eq!(parsed.global.level, "info");
/// assert_eq!(parsed.appenders.len(), 1);
/// ```
pub fn parse(content: &str) -> Result<Config, ConfigError> {
    let config: Config = toml::from_str(content)?;
    Ok(config)
}

/// Try to initialize tracing from a TOML string.
///
/// **Deprecated:** this function has the same behavior as [`init_from_str`]
/// because `init_with_config` already silently returns `Ok(())` when a
/// global dispatcher has been set. Use [`init_from_str`] instead.
#[deprecated(
    note = "Use init_from_str instead — both functions silently skip if already initialized"
)]
pub fn try_init_from_str(content: &str) -> Result<(), ConfigError> {
    let config: Config = toml::from_str(content)?;
    init_with_config(config)
}

fn init_with_config(config: Config) -> Result<(), ConfigError> {
    if tracing::dispatcher::has_been_set() {
        return Ok(());
    }

    let _ = windows::enable_ansi_escapes();

    let env_filter = build_env_filter(&config)?;

    let enabled: Vec<&AppenderConfig> = config.appenders.iter().filter(|a| a.enabled).collect();
    let rate = if config.sampling.enabled {
        config.sampling.rate_per_second
    } else {
        0
    };

    #[cfg(feature = "opentelemetry")]
    let otel_guard = {
        if config.opentelemetry.enabled {
            Some(build_otel_layer(&config.opentelemetry)?)
        } else {
            None
        }
    };

    if enabled.is_empty() {
        #[cfg(feature = "opentelemetry")]
        if let Some(guard) = otel_guard {
            let provider: &'static _ = Box::leak(Box::new(guard.provider));
            let _ = provider;
            tracing_subscriber::registry()
                .with(guard.layer)
                .with(env_filter)
                .with(SpanFieldsLayer)
                .with(tracing_subscriber::fmt::layer())
                .try_init()
                .ok();
            return Ok(());
        }

        tracing_subscriber::registry()
            .with(env_filter)
            .with(SpanFieldsLayer)
            .with(tracing_subscriber::fmt::layer())
            .try_init()
            .ok();
        return Ok(());
    }

    let mut make_writer = MultiMakeWriter::default();
    let mut first_formatter_cfg: Option<&crate::config::FormatterConfig> = None;
    for appender_cfg in &enabled {
        let factory = build_factory(appender_cfg, rate)?;
        make_writer.push(factory);
        if first_formatter_cfg.is_none() {
            first_formatter_cfg = Some(&appender_cfg.formatter);
        }
    }

    let formatter_cfg = first_formatter_cfg
        .ok_or_else(|| ConfigError::InvalidConfig("no enabled appenders".into()))?;
    let formatter = build_formatter(formatter_cfg)?;

    #[cfg(feature = "opentelemetry")]
    if let Some(guard) = otel_guard {
        let provider: &'static _ = Box::leak(Box::new(guard.provider));
        let _ = provider;
        tracing_subscriber::registry()
            .with(guard.layer)
            .with(env_filter)
            .with(SpanFieldsLayer)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(make_writer)
                    .event_format(formatter),
            )
            .try_init()
            .ok();
        return Ok(());
    }

    tracing_subscriber::registry()
        .with(env_filter)
        .with(SpanFieldsLayer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(make_writer)
                .event_format(formatter),
        )
        .try_init()
        .ok();
    Ok(())
}

/// Build a factory that produces a fresh `Box<dyn Write + Send>` per
/// event, using the configured appender.
fn build_factory(
    appender_cfg: &AppenderConfig,
    global_rate: u64,
) -> Result<WriterFactory, ConfigError> {
    let rate = if appender_sampling_enabled(appender_cfg) {
        global_rate
    } else {
        0
    };
    let factory: WriterFactory = match appender_cfg.kind.as_str() {
        "stdout" => make_sampler_factory(SamplingWriter::new(std::io::stdout, rate)),
        "stderr" => make_sampler_factory(SamplingWriter::new(std::io::stderr, rate)),
        "file" => {
            let path = appender_cfg.path.as_ref().ok_or_else(|| {
                ConfigError::InvalidConfig("file appender requires 'path'".into())
            })?;
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(appender_cfg.append)
                .open(path)?;
            make_sampler_factory(SamplingWriter::new(file, rate))
        }
        "rolling_file" => {
            let dir = appender_cfg
                .dir
                .as_ref()
                .ok_or(ConfigError::RollingMissingDir)?;
            let rotation = appender_cfg.rotation.as_deref().unwrap_or("daily");
            let rot = match rotation {
                "daily" => tracing_appender::rolling::Rotation::DAILY,
                "hourly" => tracing_appender::rolling::Rotation::HOURLY,
                "never" => tracing_appender::rolling::Rotation::NEVER,
                _ => {
                    return Err(ConfigError::InvalidConfig(format!(
                        "unknown rotation '{}'",
                        rotation
                    )));
                }
            };
            let prefix = appender_cfg.prefix.as_deref().unwrap_or("app");
            let suffix = appender_cfg.suffix.as_deref().unwrap_or("log");
            let appender = RollingFileAppender::new(rot, dir, format!("{}.{}", prefix, suffix));
            make_sampler_factory(SamplingWriter::new(appender, rate))
        }
        other => {
            return Err(ConfigError::UnknownAppenderKind {
                kind: other.to_string(),
            });
        }
    };
    Ok(factory)
}

fn appender_sampling_enabled(_appender_cfg: &AppenderConfig) -> bool {
    // Per-appender sampling config is reserved for the future. For now
    // the global rate is used for every appender.
    true
}

/// Factory used by `MultiMakeWriter` to obtain a writer per event.
type WriterFactory = Box<dyn Fn() -> Box<dyn IoWrite> + Send + Sync>;

/// Box the `SamplingWriter` so we hold a `'static` reference, then
/// install a closure that calls its `MakeWriter` impl per event.
/// `Box::leak` is acceptable here because the writer lives for the
/// remainder of the process.
fn make_sampler_factory<W>(sampler: SamplingWriter<W>) -> WriterFactory
where
    W: MakeWriter<'static> + Send + Sync + 'static,
    W::Writer: IoWrite,
{
    let sampler: &'static SamplingWriter<W> = Box::leak(Box::new(sampler));
    Box::new(move || {
        let guard = sampler.make_writer();
        Box::new(guard) as Box<dyn IoWrite>
    })
}

fn build_env_filter(config: &Config) -> Result<EnvFilter, ConfigError> {
    let default_lvl = if config.filter.default_level.is_empty() {
        config.global.level.clone()
    } else {
        config.filter.default_level.clone()
    };

    let mut directives = Vec::new();
    if !default_lvl.is_empty() {
        directives.push(default_lvl);
    }
    for d in &config.filter.directives {
        directives.push(d.clone());
    }
    for appender in &config.appenders {
        if appender.enabled {
            if let Some(lvl) = &appender.level {
                directives.push(format!("{}={}", appender.name, lvl));
            }
        }
    }

    let filter_str = directives.join(",");
    if filter_str.is_empty() {
        return EnvFilter::try_new("info").map_err(|e| ConfigError::InvalidConfig(e.to_string()));
    }
    EnvFilter::try_new(&filter_str).map_err(|e| ConfigError::InvalidConfig(e.to_string()))
}

/// A `MakeWriter` that fans out a single `write()` to a list of writers.
#[derive(Default)]
struct MultiMakeWriter {
    factories: Vec<WriterFactory>,
}

impl MultiMakeWriter {
    fn push(&mut self, f: WriterFactory) {
        self.factories.push(f);
    }
}

impl<'a> MakeWriter<'a> for MultiMakeWriter {
    type Writer = MultiWriter;
    fn make_writer(&'a self) -> Self::Writer {
        let writers: Vec<Box<dyn IoWrite>> = self.factories.iter().map(|f| f()).collect();
        MultiWriter { writers }
    }
}

struct MultiWriter {
    writers: Vec<Box<dyn IoWrite>>,
}

impl IoWrite for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for w in self.writers.iter_mut() {
            // Match the previous stdout.and(file) semantics: a failure
            // on one writer shouldn't abort the others.
            let _ = w.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        for w in self.writers.iter_mut() {
            let _ = w.flush();
        }
        Ok(())
    }
}
