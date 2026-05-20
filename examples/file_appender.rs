//! File appender example - demonstrates logging to a specific file path.

use std::fs;

fn main() {
    let log_path = "/tmp/tracing-config-demo.log";

    if fs::remove_file(log_path).is_ok() {
        println!("Removed existing log file");
    }

    let config = format!(
        r#"
[global]
level = "info"

[[appender]]
name = "file"
kind = "file"
path = "{}"
append = true
enabled = true

[appender.formatter]
type = "default"
with_time = true
with_level = true
with_target = true
"#,
        log_path
    );

    tracing_config::init_from_str(&config).expect("failed to init tracing");

    println!("Logging to file: {}", log_path);
    println!("Check the file for log output after running.\n");

    tracing::info!("Info message from tracing-config");
    tracing::warn!("Warning message");
    tracing::error!("Error message");

    std::thread::sleep(std::time::Duration::from_millis(100));

    let content = fs::read_to_string(log_path).expect("failed to read log file");
    println!("\n=== File contents ===");
    println!("{}", content);
    println!("====================\n");

    println!("SUCCESS: File appender is working!");
}
