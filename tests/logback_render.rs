//! Integration tests for M1.1-M1.4: exception rendering, %clr{color},
//! span field extraction via %X{key}, and event field rendering via %kvp.

use std::error::Error as StdError;
use std::io;
use std::sync::{Arc, Mutex};

use tracing_declarative::formatter::logback::{scan, Keyword, LogbackFormatter, Token};
use tracing_declarative::span_fields::SpanFieldsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

/// Run `body` with a thread-local subscriber that pipes its log
/// output into a captured buffer, then return the captured text.
fn capture<F: FnOnce()>(pattern: &str, body: F) -> String {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let buf_for_writer = buf.clone();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("trace"))
        .with(SpanFieldsLayer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(move || WriterForBuffer {
                    buf: buf_for_writer.clone(),
                })
                .with_ansi(false)
                .event_format(LogbackFormatter::new(scan(pattern).unwrap())),
        );

    tracing::subscriber::with_default(subscriber, body);
    let bytes = buf.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
}

struct WriterForBuffer {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl std::io::Write for WriterForBuffer {
    fn write(&mut self, src: &[u8]) -> std::io::Result<usize> {
        self.buf.lock().unwrap().extend_from_slice(src);
        Ok(src.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn exception_renders_cause_chain() {
    let out = capture("%level %msg%n%ex", || {
        let err: Box<dyn StdError + Send + Sync> =
            Box::new(io::Error::new(io::ErrorKind::Other, "operation failed"));
        tracing::error!(error = &*err, "first event");

        let chained: Box<dyn StdError + Send + Sync> = Box::new(ChainedError {
            msg: "outer".to_string(),
            source: Some(Box::new(ChainedError {
                msg: "inner".to_string(),
                source: None,
            })),
        });
        // `&*chained` derefs the Box into a `&dyn Error`, which is
        // what `record_error` needs to walk the source chain.
        tracing::error!(error = &*chained, "chained event");
    });

    assert!(out.contains("operation failed"), "got: {}", out);
    assert!(out.contains("outer"), "got: {}", out);
    assert!(out.contains("inner"), "got: {}", out);
    assert!(out.contains("Caused by"), "got: {}", out);
}

#[test]
fn nopex_suppresses_implicit_exception() {
    let out = capture("%level %msg%n%nopex", || {
        let err: Box<dyn StdError + Send + Sync> =
            Box::new(io::Error::new(io::ErrorKind::Other, "boom"));
        tracing::error!(error = &*err, "should not show error text");
    });
    assert!(!out.contains("boom"), "got: {}", out);
    assert!(!out.contains("Caused by"), "got: {}", out);
}

#[test]
fn exception_depth_limit() {
    let out = capture("%msg|%ex{1}", || {
        let chained: Box<dyn StdError + Send + Sync> = Box::new(ChainedError {
            msg: "frame1".to_string(),
            source: Some(Box::new(ChainedError {
                msg: "frame2".to_string(),
                source: Some(Box::new(ChainedError {
                    msg: "frame3".to_string(),
                    source: None,
                })),
            })),
        });
        tracing::error!(error = &*chained, "event");
    });
    assert!(out.contains("frame1"), "got: {}", out);
    assert!(!out.contains("frame2"), "got: {}", out);
    assert!(!out.contains("frame3"), "got: {}", out);
}

#[test]
fn rex_appends_package_info() {
    let out = capture("%msg|%rEx", || {
        let err: Box<dyn StdError + Send + Sync> =
            Box::new(io::Error::new(io::ErrorKind::Other, "boom"));
        tracing::error!(error = &*err, "event");
    });
    assert!(out.contains("boom"), "got: {}", out);
    assert!(out.contains("[tracing-declarative"), "got: {}", out);
    assert!(out.contains(env!("CARGO_PKG_VERSION")), "got: {}", out);
}

#[test]
fn kvp_renders_event_fields() {
    let out = capture("[%kvp]", || {
        tracing::warn!(user_id = 42, action = "login", "user logged in");
    });
    assert!(out.contains("user_id=42"), "got: {}", out);
    assert!(out.contains("action=login"), "got: {}", out);
    assert!(
        !out.contains("message="),
        "message should not appear in kvp: {}",
        out
    );
}

#[test]
fn mdc_key_renders_span_field() {
    let out = capture("rid=%X{request_id}", || {
        let span = tracing::info_span!("op", request_id = "abc-123");
        let _enter = span.enter();
        tracing::info!("hello");
    });
    assert!(out.contains("rid=abc-123"), "got: {}", out);
}

#[test]
fn mdc_missing_key_returns_empty() {
    let out = capture("rid=[%X{missing}]", || {
        tracing::info!("hello");
    });
    assert!(out.contains("rid=[]"), "got: {}", out);
}

#[test]
fn mdc_without_key_renders_all_fields() {
    let out = capture("ctx=%X", || {
        let span = tracing::info_span!("op", request_id = "abc", user_id = 7);
        let _enter = span.enter();
        tracing::info!("hi");
    });
    assert!(out.contains("ctx="), "got: {}", out);
    assert!(out.contains("request_id="), "got: {}", out);
    assert!(out.contains("user_id="), "got: {}", out);
}

#[test]
fn marker_renders_field() {
    let out = capture("%marker", || {
        tracing::info!(marker = "SECURITY", "auth event");
    });
    assert!(out.contains("SECURITY"), "got: {}", out);
}

#[test]
fn message_renders_correctly() {
    let out = capture("%msg", || {
        tracing::info!("hello world");
    });
    assert!(out.contains("hello world"), "got: {}", out);
}

#[test]
fn clr_with_color_option_parses() {
    let tokens = scan("%clr(%msg){red}").unwrap();
    assert!(matches!(
        &tokens[0],
        Token::Conversion {
            keyword: Keyword::Clr,
            option: Some(opt),
            sub_pattern: Some(_),
            ..
        } if opt == "red"
    ));
}

#[test]
fn color_words_parse_as_composite() {
    use tracing_declarative::formatter::logback::color::Color;

    for (pattern, expected) in [
        ("%red(%msg)", Color::Red),
        ("%green(%msg)", Color::Green),
        ("%yellow(%msg)", Color::Yellow),
        ("%blue(%msg)", Color::Blue),
        ("%magenta(%msg)", Color::Magenta),
        ("%cyan(%msg)", Color::Cyan),
        ("%white(%msg)", Color::White),
        ("%faint(%msg)", Color::Faint),
        ("%boldRed(%msg)", Color::BoldRed),
        ("%boldGreen(%msg)", Color::BoldGreen),
        ("%boldYellow(%msg)", Color::BoldYellow),
        ("%boldBlue(%msg)", Color::BoldBlue),
    ] {
        let tokens = scan(pattern).unwrap();
        assert!(
            matches!(
                &tokens[0],
                Token::Conversion {
                    keyword: Keyword::ColorWord(c),
                    sub_pattern: Some(_),
                    ..
                } if *c == expected
            ),
            "pattern {} did not parse as ColorWord({:?}): {:#?}",
            pattern,
            expected,
            tokens
        );
    }
}

#[test]
fn level_renders_correctly() {
    let out = capture("%level", || {
        tracing::warn!("warn-level-event");
    });
    assert!(out.contains("WARN"), "got: {}", out);
}

#[test]
fn implicit_exception_appended_when_missing() {
    let out = capture("[%level] %msg", || {
        let err: Box<dyn StdError + Send + Sync> =
            Box::new(io::Error::new(io::ErrorKind::Other, "auto-appended"));
        tracing::error!(error = &*err, "event");
    });
    // Without %ex in pattern, error text should still appear.
    assert!(out.contains("auto-appended"), "got: {}", out);
}

// === helper: chained error for testing cause chain ===

#[derive(Debug)]
struct ChainedError {
    msg: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl std::fmt::Display for ChainedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl StdError for ChainedError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn StdError + 'static))
    }
}
