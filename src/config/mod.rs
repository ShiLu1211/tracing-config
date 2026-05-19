//! Configuration structures parsed from tracing.toml.

use serde::Deserialize;
use std::path::Path;

use crate::error::ConfigError;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub global: GlobalConfig,

    #[serde(default)]
    pub filter: FilterConfig,

    #[serde(rename = "appender", default)]
    pub appenders: Vec<AppenderConfig>,

    #[serde(default)]
    pub sampling: SamplingConfig,

    #[serde(default)]
    pub opentelemetry: OpentelemetryConfig,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(Into::into)
    }

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

        Err(ConfigError::NoConfig)
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct GlobalConfig {
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default = "default_ansi")]
    pub ansi: bool,
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

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FilterConfig {
    #[serde(default = "default_default_level")]
    pub default_level: String,
    #[serde(default)]
    pub directives: Vec<String>,
}

fn default_default_level() -> String {
    "info".to_string()
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SamplingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rate")]
    pub rate_per_second: u64,
}

fn default_rate() -> u64 {
    1000
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct OpentelemetryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub service_name: String,
    #[serde(default)]
    pub service_version: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppenderConfig {
    pub name: String,
    pub kind: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub path: Option<String>,
    #[serde(default = "default_append")]
    pub append: bool,
    pub dir: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub rotation: Option<String>,
    pub max_size: Option<u64>,
    pub max_files: Option<u32>,
    pub level: Option<String>,
    #[serde(default)]
    pub formatter: FormatterConfig,
}

fn default_enabled() -> bool {
    true
}

fn default_append() -> bool {
    true
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FormatterConfig {
    #[serde(default = "default_formatter_type", rename = "type")]
    pub typ: String,
    pub pattern: Option<String>,
    #[serde(default)]
    pub compact: bool,
    #[serde(default)]
    pub pretty: bool,
    #[serde(default)]
    pub json: bool,
    #[serde(default = "default_with_target")]
    pub with_target: bool,
    #[serde(default)]
    pub with_file: bool,
    #[serde(default)]
    pub with_line: bool,
    #[serde(default)]
    pub with_thread: bool,
    #[serde(default = "default_with_level")]
    pub with_level: bool,
    #[serde(default = "default_with_time")]
    pub with_time: bool,
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
