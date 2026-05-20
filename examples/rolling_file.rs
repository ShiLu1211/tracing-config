//! Rolling file example - demonstrates rolling file appender with rotation.
//!
//! This example creates a rolling log file that rotates daily.
//! Run this example multiple times to see new log files being created.

use std::fs;
use std::path::Path;

fn main() {
    let log_dir = "/tmp/tracing-rolling";

    if !Path::new(log_dir).exists() {
        fs::create_dir_all(log_dir).expect("failed to create log directory");
    }

    let config = format!(
        r#"
[global]
level = "debug"

[filter]
default_level = "info"

[[appender]]
name = "rolling"
kind = "rolling_file"
dir = "{}"
prefix = "app"
suffix = "log"
rotation = "daily"
enabled = true

[appender.formatter]
type = "logback"
pattern = "%d{{%Y-%m-%d %H:%M:%S}} [%level][%thread] %logger - %msg%n"
"#,
        log_dir
    );

    tracing_config::init_from_str(&config).expect("failed to init tracing");

    println!("Logging to rolling file: {}/app.YYYYMMDD.log", log_dir);
    println!("Rotation: daily\n");

    tracing::info!("Application started");
    tracing::debug!("Debug message (filtered out at info level)");
    tracing::warn!("Warning: This is a warning");
    tracing::error!("Error: Something went wrong");

    std::thread::sleep(std::time::Duration::from_millis(100));

    if let Ok(entries) = fs::read_dir(log_dir) {
        println!("\n=== Log files created ===");
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                println!("  {}", name);
            }
        }
    }

    println!("\nSUCCESS: Rolling file appender is working!");
}
