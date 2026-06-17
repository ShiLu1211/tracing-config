//! Tests for rolling file appender functionality.

use tempfile::TempDir;

#[test]
fn test_rolling_daily_rotation() {
    let config = r#"
[global]
level = "info"

[[appender]]
name = "rolling"
kind = "rolling_file"
dir = "/tmp"
prefix = "test_app"
suffix = "log"
rotation = "daily"

[appender.formatter]
type = "default"
"#;
    tracing_declarative::init_from_str(config).expect("failed to init");
}

#[test]
fn test_rolling_file_write() {
    let dir = TempDir::new().unwrap();
    let config = format!(
        r#"
[global]
level = "info"

[[appender]]
name = "rolling"
kind = "rolling_file"
dir = "{}"
prefix = "test"
suffix = "log"
rotation = "never"

[appender.formatter]
type = "default"
"#,
        dir.path().display()
    );

    tracing_declarative::init_from_str(&config).expect("failed to init");
    tracing::info!("test message");
}
