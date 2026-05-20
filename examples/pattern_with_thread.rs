//! Logback pattern with thread: `%d{%H:%M:%S} [%level][%thread] %logger - %msg%n`

fn main() {
    let config = r#"
[global]
level = "info"

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "logback"
pattern = "%d{%H:%M:%S} [%level][%thread] %logger - %msg%n"
"#;

    tracing_config::init_from_str(config).expect("failed to init");

    tracing::info!("User logged in");
    tracing::warn!("Cache miss for key");
    tracing::error!("Connection timeout");
}
