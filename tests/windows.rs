//! Tests for Windows console support.

#[cfg(windows)]
#[test]
fn test_enable_ansi_escapes_windows() {
    use tracing_config::windows::enable_ansi_escapes;
    assert!(enable_ansi_escapes().is_ok());
}

#[cfg(not(windows))]
#[test]
fn test_enable_ansi_escapes_non_windows() {
    use tracing_config::windows::enable_ansi_escapes;
    assert!(enable_ansi_escapes().is_ok());
}
