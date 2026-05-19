//! Multi-appender example - stdout and file output simultaneously.

use std::fs;

fn main() {
    let log_path = "/tmp/tracing-multi.log";

    // Step 1: Clean up any existing log file
    if fs::remove_file(log_path).is_ok() {
        println!("Cleaned up existing log file at {}", log_path);
    }

    // Step 2: Create config with TWO appenders: stdout AND file
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

[[appender]]
name = "file"
kind = "file"
path = "/tmp/tracing-multi.log"
enabled = true

[appender.formatter]
type = "default"
"#;

    tracing_config::init_from_str(config).expect("failed to init tracing");

    // Step 3: Log messages at different levels
    tracing::info!("hello from tracing-config");
    tracing::warn!("this is a warning");
    tracing::error!("this is an error");
    tracing::debug!("debug message should not appear");
    tracing::trace!("trace message should not appear");

    // Step 4: Give time for flush
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Step 5: Verify file was written and contains expected messages
    let content = fs::read_to_string(log_path).expect("failed to read log file");

    println!("\n=== File content at {} ===", log_path);
    println!("{}", content);
    println!("=== End of file content ===\n");

    // Step 6: Verify expected messages are in the file
    assert!(
        content.contains("hello from tracing-config"),
        "Missing info message"
    );
    assert!(
        content.contains("this is a warning"),
        "Missing warning message"
    );
    assert!(
        content.contains("this is an error"),
        "Missing error message"
    );
    assert!(
        !content.contains("debug message"),
        "Debug message should not appear"
    );

    println!("SUCCESS: Both stdout and file appenders received output!");
    println!(
        "Verified log file contains expected messages at {}",
        log_path
    );
}
