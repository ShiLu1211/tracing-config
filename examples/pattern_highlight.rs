//! Logback pattern with highlight: `%highlight(%level) %logger - %msg%n`

fn main() {
    let config = r#"
[global]
level = "debug"

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "logback"
pattern = "%highlight(%level) %logger - %msg%n"
"#;

    tracing_config::init_from_str(config).expect("failed to init");

    println!("Log levels with color highlighting (colors may not display in all terminals):\n");

    tracing::error!("ERROR: This is an error message");
    tracing::warn!("WARN: This is a warning message");
    tracing::info!("INFO: This is an info message");
    tracing::debug!("DEBUG: This is a debug message");
}
