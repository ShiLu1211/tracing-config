use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::io::Write as IoWrite;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

use tracing_declarative::formatter::log4j::{scan as log4j_scan, Log4jFormatter};
use tracing_declarative::formatter::logback::{scan as logback_scan, LogbackFormatter};
use tracing_declarative::sampling::RateLimiter;
use tracing_declarative::span_fields::SpanFieldsLayer;

struct NullWriter;

impl IoWrite for NullWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct MakeNullWriter;

impl<'a> tracing_subscriber::fmt::writer::MakeWriter<'a> for MakeNullWriter {
    type Writer = NullWriter;
    fn make_writer(&'a self) -> Self::Writer {
        NullWriter
    }
}

fn with_logback_subscriber<F: FnOnce()>(pattern: &str, f: F) {
    let subscriber = Registry::default().with(SpanFieldsLayer).with(
        tracing_subscriber::fmt::layer()
            .with_writer(MakeNullWriter)
            .with_ansi(false)
            .event_format(LogbackFormatter::new(logback_scan(pattern).unwrap())),
    );
    tracing::subscriber::with_default(subscriber, f);
}

fn with_log4j_subscriber<F: FnOnce()>(pattern: &str, f: F) {
    let subscriber = Registry::default().with(SpanFieldsLayer).with(
        tracing_subscriber::fmt::layer()
            .with_writer(MakeNullWriter)
            .with_ansi(false)
            .event_format(Log4jFormatter::new(log4j_scan(pattern).unwrap())),
    );
    tracing::subscriber::with_default(subscriber, f);
}

fn with_default_compact_subscriber<F: FnOnce()>(f: F) {
    let subscriber = Registry::default().with(
        tracing_subscriber::fmt::layer()
            .with_writer(MakeNullWriter)
            .with_ansi(false)
            .compact(),
    );
    tracing::subscriber::with_default(subscriber, f);
}

fn with_default_full_subscriber<F: FnOnce()>(f: F) {
    let subscriber = Registry::default().with(
        tracing_subscriber::fmt::layer()
            .with_writer(MakeNullWriter)
            .with_ansi(false),
    );
    tracing::subscriber::with_default(subscriber, f);
}

fn with_declarative_default_subscriber<F: FnOnce()>(f: F) {
    let subscriber = Registry::default().with(SpanFieldsLayer).with(
        tracing_subscriber::fmt::layer()
            .with_writer(MakeNullWriter)
            .with_ansi(false)
            .compact(),
    );
    tracing::subscriber::with_default(subscriber, f);
}

// === B1: End-to-end formatting comparison ===

fn bench_b1_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("b1_e2e_formatting");
    group.throughput(Throughput::Elements(1));

    group.bench_function("tracing_default_compact", |b| {
        b.iter(|| {
            with_default_compact_subscriber(|| {
                tracing::info!("benchmark message with some payload");
            });
        });
    });

    group.bench_function("tracing_default_full", |b| {
        b.iter(|| {
            with_default_full_subscriber(|| {
                tracing::info!("benchmark message with some payload");
            });
        });
    });

    group.bench_function("declarative_default", |b| {
        b.iter(|| {
            with_declarative_default_subscriber(|| {
                tracing::info!("benchmark message with some payload");
            });
        });
    });

    group.bench_function("declarative_logback_simple", |b| {
        b.iter(|| {
            with_logback_subscriber("%d [%t] %-5level %logger{36} - %msg%n", || {
                tracing::info!("benchmark message with some payload");
            });
        });
    });

    group.bench_function("declarative_logback_full", |b| {
        b.iter(|| {
            with_logback_subscriber(
                "%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{36} - %msg%ex%n",
                || {
                    tracing::info!("benchmark message with some payload");
                },
            );
        });
    });

    group.bench_function("declarative_log4j_simple", |b| {
        b.iter(|| {
            with_log4j_subscriber("%d [%t] %-5p %c{1.} - %m%n", || {
                tracing::info!("benchmark message with some payload");
            });
        });
    });

    group.bench_function("declarative_log4j_full", |b| {
        b.iter(|| {
            with_log4j_subscriber(
                "%d{yyyy-MM-dd HH:mm:ss.SSS} [%t] %-5p %c{1.} - %m%ex%n",
                || {
                    tracing::info!("benchmark message with some payload");
                },
            );
        });
    });

    group.finish();
}

// === B2: Date formatting overhead ===

fn bench_b2_date(c: &mut Criterion) {
    let mut group = c.benchmark_group("b2_date_formatting");

    group.bench_function("chrono_local_now_format", |b| {
        let fmt = "%Y-%m-%d %H:%M:%S%.3f";
        b.iter(|| {
            let s = chrono::Local::now().format(black_box(fmt)).to_string();
            black_box(s);
        });
    });

    group.bench_function("chrono_utc_now_format", |b| {
        let fmt = "%Y-%m-%d %H:%M:%S%.3f";
        b.iter(|| {
            let s = chrono::Utc::now().format(black_box(fmt)).to_string();
            black_box(s);
        });
    });

    group.bench_function("std_system_time", |b| {
        b.iter(|| {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis();
            black_box(t);
        });
    });

    group.bench_function("chrono_local_with_timezone", |b| {
        let fmt = "%Y-%m-%d %H:%M:%S%.3f %:z";
        b.iter(|| {
            let s = chrono::Local::now().format(black_box(fmt)).to_string();
            black_box(s);
        });
    });

    group.finish();
}

// === B3: Color overhead ===

fn bench_b3_color(c: &mut Criterion) {
    let mut group = c.benchmark_group("b3_color_overhead");
    group.throughput(Throughput::Elements(1));

    group.bench_function("logback_no_color", |b| {
        b.iter(|| {
            with_logback_subscriber("%d [%t] %-5level %logger{36} - %msg%n", || {
                tracing::info!("benchmark message");
            });
        });
    });

    group.bench_function("logback_highlight", |b| {
        b.iter(|| {
            with_logback_subscriber("%d [%t] %highlight(%-5level) %logger{36} - %msg%n", || {
                tracing::info!("benchmark message");
            });
        });
    });

    group.bench_function("logback_clr", |b| {
        b.iter(|| {
            with_logback_subscriber("%d [%t] %clr(%-5level){cyan} %logger{36} - %msg%n", || {
                tracing::info!("benchmark message");
            });
        });
    });

    group.bench_function("logback_color_word", |b| {
        b.iter(|| {
            with_logback_subscriber("%d [%t] %cyan(%-5level) %logger{36} - %msg%n", || {
                tracing::info!("benchmark message");
            });
        });
    });

    group.finish();
}

// === B4: Multi-appender overhead ===

fn bench_b4_multi_appender(c: &mut Criterion) {
    let mut group = c.benchmark_group("b4_multi_appender");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single_writer", |b| {
        b.iter(|| {
            with_logback_subscriber("%level %msg%n", || {
                tracing::info!("benchmark message");
            });
        });
    });

    group.bench_function("dual_writer_sim", |b| {
        b.iter(|| {
            let subscriber = Registry::default()
                .with(SpanFieldsLayer)
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(MakeNullWriter)
                        .with_ansi(false)
                        .event_format(LogbackFormatter::new(
                            logback_scan("%level %msg%n").unwrap(),
                        )),
                )
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(MakeNullWriter)
                        .with_ansi(false)
                        .event_format(LogbackFormatter::new(
                            logback_scan("%level %msg%n").unwrap(),
                        )),
                );
            tracing::subscriber::with_default(subscriber, || {
                tracing::info!("benchmark message");
            });
        });
    });

    group.bench_function("triple_writer_sim", |b| {
        b.iter(|| {
            let subscriber = Registry::default()
                .with(SpanFieldsLayer)
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(MakeNullWriter)
                        .with_ansi(false)
                        .event_format(LogbackFormatter::new(
                            logback_scan("%level %msg%n").unwrap(),
                        )),
                )
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(MakeNullWriter)
                        .with_ansi(false)
                        .event_format(LogbackFormatter::new(
                            logback_scan("%level %msg%n").unwrap(),
                        )),
                )
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(MakeNullWriter)
                        .with_ansi(false)
                        .event_format(LogbackFormatter::new(
                            logback_scan("%level %msg%n").unwrap(),
                        )),
                );
            tracing::subscriber::with_default(subscriber, || {
                tracing::info!("benchmark message");
            });
        });
    });

    group.finish();
}

// === B5: Sampling overhead ===

fn bench_b5_sampling(c: &mut Criterion) {
    let mut group = c.benchmark_group("b5_sampling");
    group.throughput(Throughput::Elements(1));

    group.bench_function("no_sampling", |b| {
        let limiter = RateLimiter::new(0);
        b.iter(|| {
            black_box(limiter.is_allowed());
        });
    });

    group.bench_function("sampling_1000", |b| {
        let limiter = RateLimiter::new(1000);
        b.iter(|| {
            black_box(limiter.is_allowed());
        });
    });

    group.bench_function("sampling_100", |b| {
        let limiter = RateLimiter::new(100);
        b.iter(|| {
            black_box(limiter.is_allowed());
        });
    });

    group.bench_function("sampling_exhausted", |b| {
        let limiter = RateLimiter::new(1);
        limiter.is_allowed();
        b.iter(|| {
            black_box(limiter.is_allowed());
        });
    });

    group.finish();
}

// === B6: Config parsing overhead ===

fn bench_b6_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("b6_config_parsing");

    let minimal_toml = r#"
[global]
level = "info"

[[appender]]
name = "stdout"
kind = "stdout"
enabled = true

[appender.formatter]
type = "default"
"#;

    let full_toml = r#"
[global]
level = "info"
ansi = true
span_events = "none"

[filter]
default_level = "info"
directives = ["my_app::db=debug", "my_app::service=trace"]

[[appender]]
name = "stdout"
kind = "stdout"
enabled = true

[appender.formatter]
type = "logback"
pattern = "%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{36} - %msg%n"

[[appender]]
name = "file"
kind = "file"
enabled = true
path = "/tmp/app.log"
append = true

[appender.formatter]
type = "default"
json = true

[[appender]]
name = "stderr"
kind = "stderr"
enabled = false

[appender.formatter]
type = "default"
compact = true

[sampling]
enabled = true
rate_per_second = 1000

[opentelemetry]
enabled = false
endpoint = "http://localhost:4318/v1/traces"
service_name = "my-service"
"#;

    group.bench_function("parse_default_toml", |b| {
        b.iter(|| {
            let config: tracing_declarative::config::Config =
                toml::from_str(black_box(minimal_toml)).unwrap();
            black_box(config);
        });
    });

    group.bench_function("parse_full_toml", |b| {
        b.iter(|| {
            let config: tracing_declarative::config::Config =
                toml::from_str(black_box(full_toml)).unwrap();
            black_box(config);
        });
    });

    group.bench_function("lexer_logback_full_pattern", |b| {
        let pattern = "%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{36} - %msg%ex%n";
        b.iter(|| {
            let tokens = logback_scan(black_box(pattern)).unwrap();
            black_box(tokens);
        });
    });

    group.bench_function("build_formatter_logback", |b| {
        let pattern = "%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{36} - %msg%ex%n";
        b.iter(|| {
            let tokens = logback_scan(black_box(pattern)).unwrap();
            let formatter = LogbackFormatter::new(tokens);
            black_box(formatter);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_b1_formatting,
    bench_b2_date,
    bench_b3_color,
    bench_b4_multi_appender,
    bench_b5_sampling,
    bench_b6_config,
);
criterion_main!(benches);
