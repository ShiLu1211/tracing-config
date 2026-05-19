//! Tests for file appender functionality.

#[test]
fn test_file_appender_path_parsing() {
    let config = r#"
[global]
level = "info"

[[appender]]
name = "file"
kind = "file"
path = "/tmp/test_tracing.log"
append = false

[appender.formatter]
type = "default"
"#;
    let parsed = tracing_config::parse(config).expect("failed to parse");
    assert_eq!(parsed.appenders[0].kind, "file");
    assert_eq!(
        parsed.appenders[0].path.as_ref().unwrap(),
        "/tmp/test_tracing.log"
    );
    assert_eq!(parsed.appenders[0].append, false);
}

#[test]
fn test_rolling_file_config() {
    let config = r#"
[global]
level = "info"

[[appender]]
name = "rolling"
kind = "rolling_file"
dir = "/tmp"
prefix = "app"
suffix = "log"
rotation = "daily"

[appender.formatter]
type = "default"
"#;
    let parsed = tracing_config::parse(config).expect("failed to parse");
    assert_eq!(parsed.appenders[0].kind, "rolling_file");
    assert_eq!(parsed.appenders[0].dir.as_ref().unwrap(), "/tmp");
    assert_eq!(parsed.appenders[0].rotation.as_ref().unwrap(), "daily");
}
