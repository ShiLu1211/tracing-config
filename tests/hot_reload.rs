//! Tests for hot-reload functionality.
//!
//! These tests require the "hot-reload" feature to be enabled.

#[cfg(feature = "hot-reload")]
#[test]
fn test_reload_handle_new() {
    use tempfile::TempDir;
    use tracing_declarative::hot_reload::ReloadHandle;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("tracing.toml");
    std::fs::write(
        &config_path,
        r#"
[global]
level = "info"
"#,
    )
    .unwrap();

    let handle = ReloadHandle::new(&config_path);
    assert!(handle.is_ok());
}

#[cfg(feature = "hot-reload")]
#[test]
fn test_reload_handle_watch() {
    use tempfile::TempDir;
    use tracing_declarative::hot_reload::ReloadHandle;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("tracing.toml");
    std::fs::write(
        &config_path,
        r#"
[global]
level = "info"
"#,
    )
    .unwrap();

    let handle = ReloadHandle::new(&config_path).unwrap();
    assert!(handle.watch().is_ok());
}

#[cfg(feature = "hot-reload")]
#[test]
fn test_reload_handle_reload() {
    use tempfile::TempDir;
    use tracing_declarative::hot_reload::ReloadHandle;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("tracing.toml");
    std::fs::write(
        &config_path,
        r#"
[global]
level = "info"
"#,
    )
    .unwrap();

    let handle = ReloadHandle::new(&config_path).unwrap();
    assert!(handle.reload().is_ok());
}
