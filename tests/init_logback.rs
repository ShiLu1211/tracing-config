//! Tests for logback formatter initialization.

#[test]
fn test_init_logback_full() {
    let config = include_str!("fixtures/logback_full.toml");
    tracing_config::try_init_from_str(config).expect("failed to init");
}

#[test]
fn test_init_logback_color() {
    let config = include_str!("fixtures/logback_color.toml");
    tracing_config::try_init_from_str(config).expect("failed to init");
}
