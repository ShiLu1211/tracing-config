//! Hot-reload support for tracing configuration.
//!
//! **Status: unstable / best-effort.**
//!
//! The `tracing` global dispatcher is set once and cannot be
//! replaced, so this module cannot truly re-initialize the subscriber
//! on file change — `ReloadHandle::reload()` only has an effect if no
//! dispatcher has been set yet. The `notify` watcher is wired up, but
//! its callback path is a no-op for that reason.
//!
//! The proper fix is to rebase the whole appender stack on
//! `tracing_subscriber::reload::Layer` so that layer swaps are
//! possible after the initial `init()`. That refactor is tracked in
//! `docs/ROADMAP.md` (M2.6). Until then, treat the `hot-reload`
//! feature as a placeholder.

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::Config as AppConfig;
use crate::error::ConfigError;
use crate::formatter::build_formatter;

static RELOADING: AtomicBool = AtomicBool::new(false);

/// Handle to a file watcher that can trigger config reload.
///
/// **Unstable:** see module-level docs for limitations.
pub struct ReloadHandle {
    watcher: Mutex<RecommendedWatcher>,
    path: std::path::PathBuf,
}

impl ReloadHandle {
    /// Create a new watcher for the given config file path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref().to_path_buf();
        let path_clone = path.clone();

        let watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                let Ok(event) = res else { return };
                if !matches!(
                    event.kind,
                    notify::EventKind::Create(_) | notify::EventKind::Modify(_)
                ) {
                    return;
                }
                if RELOADING
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    let path_for_reload = path_clone.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = reload_config(&path_for_reload) {
                            eprintln!("Failed to reload config: {}", e);
                        }
                        RELOADING.store(false, Ordering::SeqCst);
                    });
                }
            },
            Config::default(),
        )
        .map_err(|e| ConfigError::InvalidConfig(format!("failed to create watcher: {}", e)))?;

        Ok(Self {
            watcher: Mutex::new(watcher),
            path,
        })
    }

    /// Start watching the config file for changes.
    pub fn watch(&self) -> Result<(), ConfigError> {
        self.watcher
            .lock()
            .unwrap()
            .watch(&self.path, RecursiveMode::NonRecursive)
            .map_err(|e| ConfigError::InvalidConfig(format!("failed to watch path: {}", e)))?;
        Ok(())
    }

    /// Manually reload the config from the watched file.
    pub fn reload(&self) -> Result<(), ConfigError> {
        reload_config(&self.path)
    }
}

fn reload_config(path: &Path) -> Result<(), ConfigError> {
    let config = AppConfig::from_file(path)?;
    init_with_config(config)?;
    Ok(())
}

/// Initialize tracing from a config (hot-reload variant).
///
/// **Unstable:** this is a simplified copy of `lib::init_with_config` and
/// cannot truly re-initialize the global subscriber.
pub fn init_with_config(config: AppConfig) -> Result<(), ConfigError> {
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
    env_filter: &tracing_subscriber::EnvFilter,
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
    env_filter: &tracing_subscriber::EnvFilter,
    appenders: &[&crate::config::AppenderConfig],
) -> Result<(), ConfigError> {
    use tracing_subscriber::fmt::writer::MakeWriterExt;

    let formatter = build_formatter(&appenders[0].formatter)?;

    let has_stdout = appenders.iter().any(|a| a.kind == "stdout");
    let has_file = appenders.iter().any(|a| a.kind == "file");

    if has_stdout && has_file && appenders.len() == 2 {
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
        return Err(ConfigError::InvalidConfig(
            "multi-appender supported only for stdout+file combination".into(),
        ));
    }

    Ok(())
}

fn build_env_filter(config: &AppConfig) -> Result<tracing_subscriber::EnvFilter, ConfigError> {
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
    tracing_subscriber::EnvFilter::try_new(&filter_str)
        .map_err(|e| ConfigError::InvalidConfig(e.to_string()))
}
