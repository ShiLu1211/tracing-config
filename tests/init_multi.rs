//! Tests for `init_from_str` with multi-appender configurations.

use tempfile::TempDir;

#[test]
fn init_with_stdout_and_file() {
    let dir = TempDir::new().unwrap();
    let log_path = dir.path().join("test.log");
    let config = format!(
        r#"
[global]
level = "info"

[filter]
default_level = "info"

[[appender]]
name = "stdout"
kind = "stdout"
enabled = true

[appender.formatter]
type = "logback"
pattern = "%level %msg%n"

[[appender]]
name = "file"
kind = "file"
path = "{}"
enabled = true
append = false

[appender.formatter]
type = "logback"
pattern = "[%level] %msg%n"
"#,
        log_path.display()
    );
    tracing_declarative::try_init_from_str(&config).expect("failed to init");
}

#[test]
fn init_with_stderr_and_rolling_file() {
    let dir = TempDir::new().unwrap();
    let config = format!(
        r#"
[global]
level = "info"

[[appender]]
name = "stderr"
kind = "stderr"
enabled = true

[appender.formatter]
type = "logback"
pattern = "%level %msg%n"

[[appender]]
name = "rolling"
kind = "rolling_file"
dir = "{}"
prefix = "app"
suffix = "log"
rotation = "never"
enabled = true

[appender.formatter]
type = "logback"
pattern = "%d %level %msg%n"
"#,
        dir.path().display()
    );
    tracing_declarative::try_init_from_str(&config).expect("failed to init");
}

#[test]
fn init_with_three_appenders() {
    let dir = TempDir::new().unwrap();
    let log_path = dir.path().join("three.log");
    let config = format!(
        r#"
[global]
level = "debug"

[[appender]]
name = "stdout"
kind = "stdout"
enabled = true
[appender.formatter]
type = "logback"
pattern = "[out] %msg%n"

[[appender]]
name = "stderr"
kind = "stderr"
enabled = true
[appender.formatter]
type = "logback"
pattern = "[err] %msg%n"

[[appender]]
name = "file"
kind = "file"
path = "{}"
enabled = true
append = false
[appender.formatter]
type = "logback"
pattern = "[file] %msg%n"
"#,
        log_path.display()
    );
    tracing_declarative::try_init_from_str(&config).expect("failed to init");
}
