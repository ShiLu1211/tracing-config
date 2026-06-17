//! Logback pattern full: `%d{%Y-%m-%d %H:%M:%S} [%level][%thread][%pid] %logger{36} %line - %msg%n`

fn main() {
    let config = r#"
[global]
level = "debug"

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "logback"
pattern = "%d{%Y-%m-%d %H:%M:%S} [%level][%thread][%pid] %logger{36} %line - %msg%n"
"#;

    tracing_declarative::init_from_str(config).expect("failed to init");

    tracing::info!("User logged in");
    tracing::debug!("Database query executed");
    tracing::warn!("Cache miss for key");
    tracing::error!("Connection timeout");
}
