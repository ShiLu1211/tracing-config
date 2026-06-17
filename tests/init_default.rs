//! Tests for default formatter initialization.

#[test]
fn test_init_default() {
    let config = include_str!("fixtures/default.toml");
    tracing_declarative::init_from_str(config).expect("failed to init tracing");
}

#[test]
fn test_parse_default() {
    let config = include_str!("fixtures/default.toml");
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert_eq!(parsed.global.level, "info");
    assert_eq!(parsed.filter.default_level, "info");
    assert_eq!(parsed.appenders.len(), 1);
    assert_eq!(parsed.appenders[0].name, "stdout");
}

#[test]
fn test_multiple_appenders() {
    let config = r#"
[global]
level = "debug"

[filter]
default_level = "info"
directives = ["my_app=debug"]

[[appender]]
name = "stdout"
kind = "stdout"
enabled = true

[appender.formatter]
type = "default"
compact = true
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert_eq!(parsed.global.level, "debug");
    assert_eq!(parsed.filter.directives.len(), 1);
}

#[test]
fn test_empty_appenders() {
    let config = r#"
[global]
level = "warn"

[filter]
default_level = "warn"
"#;
    let parsed = tracing_declarative::parse(config).expect("failed to parse");
    assert!(parsed.appenders.is_empty());
}
