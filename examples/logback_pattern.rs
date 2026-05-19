//! Logback pattern example - demonstrates various conversion words.
//!
//! This example showcases three different logback patterns with increasing complexity.
//!
//! # Patterns Demonstrated
//!
//! 1. **Simple Pattern** - `%level %logger - %msg%n`
//!    Basic level and logger output
//!
//! 2. **With Timestamp** - `%d{%Y-%m-%d %H:%M:%S} [%level] %logger{36} - %msg%n`
//!    Adds date formatting and logger abbreviation
//!
//! 3. **With Thread** - `%d{%H:%M:%S} [%level][%thread] %logger - %msg%n`
//!    Adds thread name for multi-threaded context

fn main() {
    let configs = vec![
        (
            "Simple Pattern: %level %logger - %msg%n",
            r#"
[global]
level = "info"

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "logback"
pattern = "%level %logger - %msg%n"
"#,
        ),
        (
            "With Timestamp: %d{%Y-%m-%d %H:%M:%S} [%level] %logger{36} - %msg%n",
            r#"
[global]
level = "info"

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "logback"
pattern = "%d{%Y-%m-%d %H:%M:%S} [%level] %logger{36} - %msg%n"
"#,
        ),
        (
            "With Thread: %d{%H:%M:%S} [%level][%thread] %logger - %msg%n",
            r#"
[global]
level = "info"

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "logback"
pattern = "%d{%H:%M:%S} [%level][%thread] %logger - %msg%n"
"#,
        ),
    ];

    for (name, config) in configs {
        println!("\n=== {} ===\n", name);

        // Re-init tracing with the new config
        // Note: In production, you'd typically call init_from_str once
        // Here we re-init to show different patterns
        tracing_config::init_from_str(config).expect("failed to init tracing");

        tracing::info!("This is an info message");
        tracing::debug!("This is a debug message");
        tracing::warn!("This is a warning message");
    }

    println!("\n=== Demo Complete ===\n");
}
