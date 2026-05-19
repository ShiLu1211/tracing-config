//! Basic example - minimal tracing-config usage.

fn main() {
    let config = r#"
[global]
level = "info"

[filter]
default_level = "info"

[[appender]]
name = "stdout"
kind = "stdout"
enabled = true

[appender.formatter]
type = "default"
"#;

    tracing_config::init_from_str(config).expect("failed to init tracing");

    tracing::info!("hello from tracing-config");
    tracing::warn!("this is a warning");
    tracing::error!("this is an error");
}
