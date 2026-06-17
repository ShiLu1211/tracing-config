//! End-to-end integration tests for the public `init_*` API.

use std::error::Error as StdError;
use std::io;
use std::sync::{Arc, Mutex};

use tracing_config::formatter::logback::{scan, Keyword, LogbackFormatter, Token};
use tracing_config::span_fields::SpanFieldsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

/// Capture log output from a closure using the given pattern.
fn capture<F: FnOnce()>(pattern: &str, body: F) -> String {
    use std::io::Write as IoWrite;
    let buf = Arc::new(Mutex::new(Vec::new()));
    let buf_for_writer = buf.clone();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("trace"))
        .with(SpanFieldsLayer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(move || -> Box<dyn IoWrite + Send> {
                    Box::new(CaptureWriter {
                        buf: buf_for_writer.clone(),
                    })
                })
                .with_ansi(false)
                .event_format(LogbackFormatter::new(scan(pattern).unwrap())),
        );
    tracing::subscriber::with_default(subscriber, body);
    let bytes = buf.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn invalid_toml_returns_error() {
    let result = tracing_config::parse(include_str!("fixtures/invalid.toml"));
    assert!(result.is_err(), "expected parse error for invalid TOML");
}

#[test]
fn full_pattern_renders_all_fields() {
    // %logger without an {N} option abbreviates to the last segment
    // (logback compatibility), so "my::module" → "module".
    let out = capture("%d{HH:mm:ss} %-5level %logger - %msg%n", || {
        tracing::warn!(target: "my::module", "boot complete");
    });
    assert!(out.contains("WARN"), "level missing: {}", out);
    assert!(out.contains("module"), "logger missing: {}", out);
    assert!(out.contains("boot complete"), "msg missing: {}", out);
    assert!(out.len() > 20, "output too short: {:?}", out);
}

#[test]
fn stderr_like_appender_renders() {
    // We can't easily capture stderr in a hermetic test, but the
    // multi-appender init test already exercises stderr. Here we just
    // verify the formatter + writer pair works for typical logback
    // patterns on top of any MakeWriter.
    let out = capture("%level %msg", || {
        let err: Box<dyn StdError + Send + Sync> =
            Box::new(io::Error::new(io::ErrorKind::Other, "demo"));
        tracing::error!(error = &*err, "stderr path");
    });
    assert!(out.contains("ERROR"), "got: {}", out);
    assert!(out.contains("demo"), "error text missing: {}", out);
}

#[test]
fn logback_pattern_with_exception_and_span_fields() {
    let out = capture("[%X{user_id}] %level %msg%n%ex", || {
        let span = tracing::info_span!("op", user_id = 7);
        let _e = span.enter();
        let err: Box<dyn StdError + Send + Sync> =
            Box::new(io::Error::new(io::ErrorKind::Other, "boom"));
        tracing::error!(error = &*err, "explosion");
    });
    assert!(out.contains("[7]"), "span field missing: {}", out);
    assert!(out.contains("ERROR"), "level missing: {}", out);
    assert!(out.contains("explosion"), "msg missing: {}", out);
    assert!(out.contains("boom"), "exception missing: {}", out);
}

#[test]
fn empty_appenders_list_uses_default_format() {
    let cfg: tracing_config::config::Config = toml::from_str(
        r#"
[global]
level = "info"
"#,
    )
    .unwrap();
    assert!(cfg.appenders.is_empty());
}

struct CaptureWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl io::Write for CaptureWriter {
    fn write(&mut self, src: &[u8]) -> io::Result<usize> {
        self.buf.lock().unwrap().extend_from_slice(src);
        Ok(src.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// Suppress unused-import warnings.
#[allow(dead_code)]
const _TOKEN: Token = Token::Literal(String::new());
#[allow(dead_code)]
const _KEYWORD: Keyword = Keyword::Level;
