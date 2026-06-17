//! Logback pattern - Simple: `%level %logger - %msg%n`

fn main() {
    let config = r#"
[global]
level = "info"

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "logback"
pattern = "%level %logger - %msg%n"
"#;

    tracing_declarative::init_from_str(config).expect("failed to init");

    tracing::info!("User logged in");
    tracing::warn!("Cache miss for key");
    tracing::error!("Connection timeout");
}
