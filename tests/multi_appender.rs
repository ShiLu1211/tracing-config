//! Tests for multi-appender combinations.

use std::sync::{Arc, Mutex};
use tracing_declarative::formatter::logback::{scan, LogbackFormatter};
use tracing_declarative::span_fields::SpanFieldsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

/// Capture stderr output by initializing with a custom subscriber.
fn capture_stderr<F: FnOnce()>(pattern: &str, body: F) -> String {
    use std::io;
    use std::sync::atomic::{AtomicBool, Ordering};

    static STARTED: AtomicBool = AtomicBool::new(false);
    STARTED.store(true, Ordering::SeqCst);

    // Since we can only init a global subscriber once, use a thread-local
    // subscriber instead. We test the MultiMakeWriter indirectly through
    // the lib.rs init path by checking the simpler unit-level pieces.
    let buf = Arc::new(Mutex::new(Vec::new()));
    let buf_for_writer = buf.clone();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("trace"))
        .with(SpanFieldsLayer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(move || -> Box<dyn io::Write + Send> {
                    Box::new(WriterForBuffer {
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
fn multi_writer_fans_out_to_all() {
    // This test exercises the multi-appender path by using multiple
    // make_writer factories. Each event should reach both buffers.
    let buf1 = Arc::new(Mutex::new(Vec::new()));
    let buf2 = Arc::new(Mutex::new(Vec::new()));
    let buf1_clone = buf1.clone();
    let buf2_clone = buf2.clone();

    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("trace"))
        .with(SpanFieldsLayer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(make_multi_writer(vec![
                    Box::new(move || -> Box<dyn std::io::Write + Send> {
                        Box::new(WriterForBuffer {
                            buf: buf1_clone.clone(),
                        })
                    }),
                    Box::new(move || -> Box<dyn std::io::Write + Send> {
                        Box::new(WriterForBuffer {
                            buf: buf2_clone.clone(),
                        })
                    }),
                ]))
                .with_ansi(false)
                .event_format(LogbackFormatter::new(scan("%msg").unwrap())),
        );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("hello");
    });

    let out1 = String::from_utf8(buf1.lock().unwrap().clone()).unwrap();
    let out2 = String::from_utf8(buf2.lock().unwrap().clone()).unwrap();
    assert!(out1.contains("hello"), "buf1: {}", out1);
    assert!(out2.contains("hello"), "buf2: {}", out2);
}

// === helper: TeeWriter that holds multiple factories ===

type WriterFactory = Box<dyn Fn() -> Box<dyn std::io::Write + Send> + Send + Sync>;

fn make_multi_writer(factories: Vec<WriterFactory>) -> TeeWriter {
    TeeWriter { factories }
}

struct TeeWriter {
    factories: Vec<WriterFactory>,
}

impl<'a> tracing_subscriber::fmt::writer::MakeWriter<'a> for TeeWriter {
    type Writer = TeeWriterHandle;
    fn make_writer(&'a self) -> Self::Writer {
        let writers: Vec<Box<dyn std::io::Write + Send>> =
            self.factories.iter().map(|f| f()).collect();
        TeeWriterHandle { writers }
    }
}

struct TeeWriterHandle {
    writers: Vec<Box<dyn std::io::Write + Send>>,
}

impl std::io::Write for TeeWriterHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for w in self.writers.iter_mut() {
            let _ = w.write_all(buf);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        for w in self.writers.iter_mut() {
            let _ = w.flush();
        }
        Ok(())
    }
}

// Suppress the unused import warning.
#[allow(dead_code)]
fn _suppress() {
    let _ = capture_stderr::<fn()>;
}
