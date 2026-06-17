//! Color output example - demonstrates ANSI color formatting.

fn main() {
    let config = r#"
[global]
level = "debug"
ansi = true

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "logback"
pattern = "%highlight(%level) %logger - %msg%n"
"#;

    tracing_declarative::init_from_str(config).expect("failed to init");

    println!("Log levels with color highlighting:");
    println!("(Colors may not display in all terminals)");
    tracing::error!("ERROR: This is an error message");
    tracing::warn!("WARN: This is a warning message");
    tracing::info!("INFO: This is an info message");
    tracing::debug!("DEBUG: This is a debug message");
    tracing::trace!("TRACE: This is a trace message");
}
