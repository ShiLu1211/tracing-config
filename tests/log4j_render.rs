//! Integration tests for the log4j formatter engine.

use std::io;
use std::sync::{Arc, Mutex};

use tracing_config::formatter::log4j::{scan, Log4jFormatter};
use tracing_config::span_fields::SpanFieldsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

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
                .event_format(Log4jFormatter::new(scan(pattern).unwrap())),
        );
    tracing::subscriber::with_default(subscriber, body);
    let bytes = buf.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
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

#[test]
fn log4j_basic_pattern() {
    let out = capture("%d{HH:mm:ss} %-5p %c{1.} - %m%n", || {
        tracing::warn!(target: "com.example.foo.Bar", "boot complete");
    });
    assert!(out.contains("WARN"), "level: {}", out);
    // `%c{1.}` abbreviates every leading segment to its first
    // letter, keeping the last segment in full.
    assert!(out.contains("c.e.f.Bar"), "logger: {}", out);
    assert!(out.contains("boot complete"), "msg: {}", out);
}

#[test]
fn log4j_percent_x_renders_span_name() {
    let out = capture("[%x] %m", || {
        let span = tracing::info_span!("op-name");
        let _e = span.enter();
        tracing::info!("hello");
    });
    assert!(out.contains("[op-name]"), "got: {}", out);
    assert!(out.contains("hello"), "got: {}", out);
}

#[test]
fn log4j_exception_renders_chain() {
    let out = capture("%p %m%n%xEx", || {
        let err: Box<dyn std::error::Error + Send + Sync> = Box::new(io::Error::other("boom"));
        tracing::error!(error = &*err, "explosion");
    });
    assert!(out.contains("ERROR"), "level: {}", out);
    assert!(out.contains("boom"), "exception: {}", out);
}

#[test]
fn log4j_enc_html_escapes_message() {
    let out = capture("%enc{%m}{html}", || {
        tracing::info!("<script>alert(\"x\")</script>");
    });
    assert!(out.contains("&lt;script&gt;"));
    assert!(out.contains("&quot;"));
    assert!(!out.contains("<script>"));
}

#[test]
fn log4j_maxlen_truncates() {
    let out = capture("%maxLen{%m}{5}", || {
        tracing::info!("hello world");
    });
    assert!(out.contains("hello"), "got: {}", out);
    assert!(!out.contains("world"), "should truncate at 5: {}", out);
}

#[test]
fn log4j_highlight_levels() {
    let out = capture("%highlight{%p}", || {
        tracing::error!("e");
    });
    // ANSI codes are stripped; just check the level word is there.
    assert!(out.contains("ERROR"), "got: {}", out);
}
