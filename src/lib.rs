//! tracing-config — Declarative tracing initialization via `tracing.toml`.

use std::path::Path;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::error::ConfigError;
use crate::formatter::build_formatter;

pub mod appender;
pub mod config;
pub mod error;
pub mod formatter;

// Build-time version info injected by build.rs
include!(concat!(env!("OUT_DIR"), "/version.rs"));

/// Initialize tracing from the default `tracing.toml` search path.
pub fn init() -> Result<(), ConfigError> {
    let config = Config::from_default_file()?;
    init_with_config(config)
}

/// Initialize tracing from a specific file path.
pub fn init_from_file(path: impl AsRef<Path>) -> Result<(), ConfigError> {
    let config = Config::from_file(path)?;
    init_with_config(config)
}

/// Initialize tracing from a TOML string.
pub fn init_from_str(content: &str) -> Result<(), ConfigError> {
    let config: Config = toml::from_str(content)?;
    init_with_config(config)
}

/// Parse configuration without initializing tracing.
pub fn parse(content: &str) -> Result<Config, ConfigError> {
    let config: Config = toml::from_str(content)?;
    Ok(config)
}

/// Try to initialize tracing, ignoring errors if already initialized.
pub fn try_init_from_str(content: &str) -> Result<(), ConfigError> {
    let config: Config = toml::from_str(content)?;
    init_with_config(config)
}

fn init_with_config(config: Config) -> Result<(), ConfigError> {
    if tracing::dispatcher::has_been_set() {
        return Ok(());
    }

    let env_filter = build_env_filter(&config)?;

    let enabled: Vec<_> = config.appenders.iter().filter(|a| a.enabled).collect();

    match enabled.len() {
        0 => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer())
                .try_init()
                .ok();
        }
        1 => {
            let appender_cfg = enabled[0];
            init_single_appender(&env_filter, appender_cfg)?;
        }
        _ => {
            init_multi_appender(&env_filter, &enabled)?;
        }
    }

    Ok(())
}

fn init_single_appender(
    env_filter: &EnvFilter,
    appender_cfg: &crate::config::AppenderConfig,
) -> Result<(), ConfigError> {
    let formatter = build_formatter(&appender_cfg.formatter)?;
    match appender_cfg.kind.as_str() {
        "stdout" => {
            tracing_subscriber::registry()
                .with(env_filter.clone())
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(std::io::stdout)
                        .event_format(formatter),
                )
                .try_init()
                .ok();
        }
        "stderr" => {
            tracing_subscriber::registry()
                .with(env_filter.clone())
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(std::io::stderr)
                        .event_format(formatter),
                )
                .try_init()
                .ok();
        }
        "file" => {
            let path = appender_cfg.path.as_ref().ok_or_else(|| {
                ConfigError::InvalidConfig("file appender requires 'path'".into())
            })?;
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(appender_cfg.append)
                .open(path)?;
            tracing_subscriber::registry()
                .with(env_filter.clone())
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(file)
                        .event_format(formatter),
                )
                .try_init()
                .ok();
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
            let appender = tracing_appender::rolling::RollingFileAppender::new(
                rot,
                dir,
                format!("{}.{}", prefix, suffix),
            );
            tracing_subscriber::registry()
                .with(env_filter.clone())
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(appender)
                        .event_format(formatter),
                )
                .try_init()
                .ok();
        }
        _ => {
            return Err(ConfigError::UnknownAppenderKind {
                kind: appender_cfg.kind.clone(),
            });
        }
    }
    Ok(())
}

fn init_multi_appender(
    env_filter: &EnvFilter,
    appenders: &[&crate::config::AppenderConfig],
) -> Result<(), ConfigError> {
    // Use a Vec of writers approach - write to multiple outputs
    use tracing_subscriber::fmt::writer::MakeWriterExt;

    // For multi-appender, we need a custom writer that broadcasts to multiple outputs
    // Simple approach: if we have stdout + file, use Tee
    // Otherwise just use the first appender's writer

    let formatter = build_formatter(&appenders[0].formatter)?;

    // Check if we have exactly stdout + file combo
    let has_stdout = appenders.iter().any(|a| a.kind == "stdout");
    let has_file = appenders.iter().any(|a| a.kind == "file");

    if has_stdout && has_file && appenders.len() == 2 {
        // Use Tee to combine stdout and file
        let file_appender = appenders.iter().find(|a| a.kind == "file").unwrap();
        let path = file_appender
            .path
            .as_ref()
            .ok_or_else(|| ConfigError::InvalidConfig("file appender requires 'path'".into()))?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(file_appender.append)
            .open(path)?;

        let writer = std::io::stdout.and(file);

        tracing_subscriber::registry()
            .with(env_filter.clone())
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(writer)
                    .event_format(formatter),
            )
            .try_init()
            .ok();
    } else {
        // Fall back to first appender only (could be enhanced later)
        return Err(ConfigError::InvalidConfig(
            "multi-appender supported only for stdout+file combination".into(),
        ));
    }

    Ok(())
}

fn build_env_filter(config: &Config) -> Result<EnvFilter, ConfigError> {
    // Use global.level as fallback, then filter directives
    let default_lvl = if config.filter.default_level.is_empty() {
        config.global.level.clone()
    } else {
        config.filter.default_level.clone()
    };

    let filter_str = if config.filter.directives.is_empty() {
        default_lvl
    } else {
        let all: Vec<_> = std::iter::once(default_lvl.as_str())
            .chain(config.filter.directives.iter().map(|s| s.as_str()))
            .collect();
        all.join(",")
    };
    EnvFilter::try_new(&filter_str).map_err(|e| ConfigError::InvalidConfig(e.to_string()))
}
