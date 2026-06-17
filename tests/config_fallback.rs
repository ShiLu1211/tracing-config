//! Tests for `Config::from_default_file` and the built-in fallback.

use tracing_config::config::Config;

#[test]
fn builtin_default_matches_spec() {
    let cfg = Config::builtin_default();
    assert_eq!(cfg.global.level, "info");
    assert_eq!(cfg.global.ansi, true);
    assert_eq!(cfg.filter.default_level, "info");
    assert!(cfg.filter.directives.is_empty());
    assert_eq!(cfg.appenders.len(), 1);
    assert_eq!(cfg.appenders[0].name, "stdout");
    assert_eq!(cfg.appenders[0].kind, "stdout");
    assert!(cfg.appenders[0].enabled);
    assert!(!cfg.sampling.enabled);
    assert!(!cfg.opentelemetry.enabled);
}

#[test]
fn builtin_default_formatter_is_default_type() {
    let cfg = Config::builtin_default();
    let f = &cfg.appenders[0].formatter;
    assert_eq!(f.typ, "default");
    assert!(f.pattern.is_none());
}

#[test]
fn from_str_parsed_config_overrides_defaults() {
    let cfg: Config = toml::from_str(
        r#"
[global]
level = "debug"
ansi = false

[filter]
default_level = "trace"

[[appender]]
name = "stderr"
kind = "stderr"
level = "warn"

[appender.formatter]
type = "logback"
pattern = "%msg"
"#,
    )
    .unwrap();
    assert_eq!(cfg.global.level, "debug");
    assert!(!cfg.global.ansi);
    assert_eq!(cfg.filter.default_level, "trace");
    assert_eq!(cfg.appenders[0].kind, "stderr");
    assert_eq!(cfg.appenders[0].level.as_deref(), Some("warn"));
    assert_eq!(cfg.appenders[0].formatter.typ, "logback");
}
