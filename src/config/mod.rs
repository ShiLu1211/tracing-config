//! Configuration structures parsed from `tracing.toml`.
//!
//! # Complete TOML example
//!
//! ```toml
//! [global]
//! level = "info"
//! ansi = true
//! span_events = "none"
//!
//! [filter]
//! default_level = "info"
//! directives = ["my_app::db=debug"]
//!
//! [[appender]]
//! name = "stdout"
//! kind = "stdout"
//! enabled = true
//!
//! [appender.formatter]
//! type = "logback"
//! pattern = "%d [%thread] %-5level %logger{36} - %msg%n"
//!
//! [[appender]]
//! name = "file"
//! kind = "rolling_file"
//! enabled = true
//! dir = "./logs"
//! prefix = "app"
//! suffix = "log"
//! rotation = "daily"
//!
//! [appender.formatter]
//! type = "default"
//! json = true
//!
//! [sampling]
//! enabled = false
//! rate_per_second = 1000
//!
//! [opentelemetry]
//! enabled = false
//! endpoint = "http://localhost:4318/v1/traces"
//! service_name = "my-service"
//! service_version = "1.0.0"
//! ```

use serde::Deserialize;
use std::path::Path;

use crate::error::ConfigError;

/// Top-level configuration root parsed from `tracing.toml`.
///
/// Deserialized via [`serde`] from the TOML content. Use [`Config::from_file`]
/// or [`Config::from_default_file`] to load, or [`crate::parse`] for the
/// convenience wrapper.
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
/// "#;
/// let parsed: tracing_declarative::config::Config = toml::from_str(config).unwrap();
/// assert_eq!(parsed.global.level, "debug");
/// ```
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    /// Global settings (level, ansi, span_events).
    #[serde(default)]
    pub global: GlobalConfig,

    /// Filter directives (env-filter style).
    #[serde(default)]
    pub filter: FilterConfig,

    /// Appender definitions (array of `[[appender]]`).
    #[serde(rename = "appender", default)]
    pub appenders: Vec<AppenderConfig>,

    /// Sampling / rate-limiting settings.
    #[serde(default)]
    pub sampling: SamplingConfig,

    /// OpenTelemetry export settings.
    #[serde(default)]
    pub opentelemetry: OpentelemetryConfig,
}

impl Config {
    /// Parse configuration from a file at the given path.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = tracing_declarative::config::Config::from_file("tracing.toml")
    ///     .expect("failed to load config");
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(Into::into)
    }

    /// Search the default config locations and return the parsed config.
    ///
    /// Search order:
    /// 1. `$TRACING_CONFIG` environment variable
    /// 2. `./tracing.toml` (current working directory)
    /// 3. `<exe-dir>/tracing.toml`
    ///
    /// If no file is found, the built-in default is returned silently
    /// (INFO level, stdout, default formatter). A file that exists but
    /// is malformed still returns `Err`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = tracing_declarative::config::Config::from_default_file()
    ///     .expect("config error");
    /// ```
    pub fn from_default_file() -> Result<Self, ConfigError> {
        if let Ok(path) = std::env::var("TRACING_CONFIG") {
            return Self::from_file(path);
        }

        let search_paths = [std::path::Path::new("./tracing.toml")];
        for path in &search_paths {
            if path.exists() {
                return Self::from_file(path);
            }
        }

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(dir) = exe_path.parent() {
                let candidate = dir.join("tracing.toml");
                if candidate.exists() {
                    return Self::from_file(candidate);
                }
            }
        }

        Ok(Self::builtin_default())
    }

    /// Built-in fallback used when no `tracing.toml` can be located.
    ///
    /// INFO-level events to stdout via the default tracing-subscriber
    /// formatter.
    ///
    /// # Example
    ///
    /// ```
    /// let config = tracing_declarative::config::Config::builtin_default();
    /// assert_eq!(config.global.level, "info");
    /// assert_eq!(config.appenders.len(), 1);
    /// assert_eq!(config.appenders[0].kind, "stdout");
    /// ```
    pub fn builtin_default() -> Self {
        Self {
            global: GlobalConfig {
                level: "info".to_string(),
                ansi: true,
                span_events: "none".to_string(),
            },
            filter: FilterConfig {
                default_level: "info".to_string(),
                directives: Vec::new(),
            },
            appenders: vec![AppenderConfig {
                name: "stdout".to_string(),
                kind: "stdout".to_string(),
                enabled: true,
                path: None,
                append: true,
                dir: None,
                prefix: None,
                suffix: None,
                rotation: None,
                max_size: None,
                max_files: None,
                level: None,
                formatter: FormatterConfig {
                    typ: "default".to_string(),
                    pattern: None,
                    compact: false,
                    pretty: false,
                    json: false,
                    with_target: true,
                    with_file: false,
                    with_line: false,
                    with_thread: false,
                    with_level: true,
                    with_time: true,
                    time_format: "%Y-%m-%dT%H:%M:%S%.3f".to_string(),
                },
            }],
            sampling: SamplingConfig {
                enabled: false,
                rate_per_second: 1000,
            },
            opentelemetry: OpentelemetryConfig::default(),
        }
    }
}

/// Global settings (`[global]` section).
#[derive(Debug, Deserialize, Default, Clone)]
pub struct GlobalConfig {
    /// Minimum log level: trace | debug | info | warn | error | off.
    #[serde(default = "default_level")]
    pub level: String,
    /// Whether to enable ANSI color codes in output.
    #[serde(default = "default_ansi")]
    pub ansi: bool,
    /// Span event capture strategy: none | new | enter | exit | close | full.
    #[serde(default = "default_span_events")]
    pub span_events: String,
}

fn default_level() -> String {
    "info".to_string()
}

fn default_ansi() -> bool {
    true
}

fn default_span_events() -> String {
    "none".to_string()
}

/// Filter directives (`[filter]` section), compatible with `RUST_LOG` syntax.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct FilterConfig {
    /// Default level when no directive matches.
    #[serde(default = "default_default_level")]
    pub default_level: String,
    /// Additional filter directives (e.g. `my_app::db=trace`).
    #[serde(default)]
    pub directives: Vec<String>,
}

fn default_default_level() -> String {
    "info".to_string()
}

/// Sampling / rate-limiting configuration (`[sampling]` section).
#[derive(Debug, Deserialize, Default, Clone)]
pub struct SamplingConfig {
    /// Whether sampling is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Maximum events per second per appender.
    #[serde(default = "default_rate")]
    pub rate_per_second: u64,
}

fn default_rate() -> u64 {
    1000
}

/// OpenTelemetry export configuration (`[opentelemetry]` section).
///
/// Only used when the `opentelemetry` feature is enabled.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct OpentelemetryConfig {
    /// Whether to enable OpenTelemetry trace export.
    #[serde(default)]
    pub enabled: bool,
    /// OTLP collector endpoint (e.g. `http://localhost:4317`).
    #[serde(default)]
    pub endpoint: String,
    /// Service name reported to the collector.
    #[serde(default)]
    pub service_name: String,
    /// Service version reported to the collector.
    #[serde(default)]
    pub service_version: String,
}

/// Appender definition (`[[appender]]` section).
#[derive(Debug, Deserialize, Clone)]
pub struct AppenderConfig {
    /// Logical name (used in filter directives).
    pub name: String,
    /// Kind of appender: stdout | stderr | file | rolling_file.
    pub kind: String,
    /// Whether this appender is active.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// File path (required for `kind = "file"`).
    pub path: Option<String>,
    /// Whether to append to an existing file (default: true).
    #[serde(default = "default_append")]
    pub append: bool,
    /// Directory for rolling file appender.
    pub dir: Option<String>,
    /// Filename prefix for rolling file appender.
    pub prefix: Option<String>,
    /// Filename suffix for rolling file appender.
    pub suffix: Option<String>,
    /// Rotation strategy: daily | hourly | never.
    pub rotation: Option<String>,
    /// Max file size in bytes (for size-based rotation, not yet implemented).
    pub max_size: Option<u64>,
    /// Max number of rotated files to keep.
    pub max_files: Option<u32>,
    /// Override log level for this appender.
    pub level: Option<String>,
    /// Formatter configuration for this appender.
    #[serde(default)]
    pub formatter: FormatterConfig,
}

fn default_enabled() -> bool {
    true
}

fn default_append() -> bool {
    true
}

/// Formatter configuration (`[appender.formatter]` section).
#[derive(Debug, Deserialize, Default, Clone)]
pub struct FormatterConfig {
    /// Formatter engine: default | logback | log4j | pattern (alias for logback).
    #[serde(default = "default_formatter_type", rename = "type")]
    pub typ: String,
    /// Format pattern (required for logback/log4j engines).
    pub pattern: Option<String>,
    /// Use compact single-line format (default engine only).
    #[serde(default)]
    pub compact: bool,
    /// Use pretty multi-line format (default engine only).
    #[serde(default)]
    pub pretty: bool,
    /// Use JSON structured format (default engine only).
    #[serde(default)]
    pub json: bool,
    /// Include target/module path in output.
    #[serde(default = "default_with_target")]
    pub with_target: bool,
    /// Include source file name in output.
    #[serde(default)]
    pub with_file: bool,
    /// Include source line number in output.
    #[serde(default)]
    pub with_line: bool,
    /// Include thread name in output.
    #[serde(default)]
    pub with_thread: bool,
    /// Include log level in output.
    #[serde(default = "default_with_level")]
    pub with_level: bool,
    /// Include timestamp in output.
    #[serde(default = "default_with_time")]
    pub with_time: bool,
    /// Timestamp strftime format string.
    #[serde(default = "default_time_format")]
    pub time_format: String,
}

fn default_formatter_type() -> String {
    "default".to_string()
}

fn default_with_target() -> bool {
    true
}

fn default_with_level() -> bool {
    true
}

fn default_with_time() -> bool {
    true
}

fn default_time_format() -> String {
    "%Y-%m-%dT%H:%M:%S%.3f".to_string()
}
