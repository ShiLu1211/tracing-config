use criterion::{black_box, criterion_group, criterion_main, Criterion};

use tracing_declarative::formatter::log4j::lexer::scan as log4j_scan;
use tracing_declarative::formatter::logback::lexer::scan as logback_scan;

fn bench_lexer(c: &mut Criterion) {
    let logback_pattern = "%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{36} - %msg%n";
    let log4j_pattern = "%d{yyyy-MM-dd HH:mm:ss.SSS} [%t] %-5p %c{1.} - %m%n";

    c.bench_function("logback_lexer_scan", |b| {
        b.iter(|| {
            let tokens = logback_scan(black_box(logback_pattern));
            black_box(tokens);
        });
    });

    c.bench_function("log4j_lexer_scan", |b| {
        b.iter(|| {
            let tokens = log4j_scan(black_box(log4j_pattern));
            black_box(tokens);
        });
    });
}

fn bench_date_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("date_format");

    group.bench_function("preprocessed_chrono", |b| {
        let chrono_fmt = "%Y-%m-%d %H:%M:%S%.3f";
        b.iter(|| {
            let s = chrono::Local::now()
                .format(black_box(chrono_fmt))
                .to_string();
            black_box(s);
        });
    });

    group.bench_function("convert_pattern_per_call", |b| {
        let java_fmt = "yyyy-MM-dd HH:mm:ss.SSS";
        b.iter(|| {
            let chrono_fmt =
                tracing_declarative::formatter::logback::date::convert_pattern(black_box(java_fmt));
            let s = chrono::Local::now().format(&chrono_fmt).to_string();
            black_box(s);
        });
    });

    group.finish();
}

fn bench_abbreviator(c: &mut Criterion) {
    let name = "my_app::service::user_handler::create_user";

    c.bench_function("logback_abbreviate_20", |b| {
        b.iter(|| {
            let s = tracing_declarative::formatter::logback::abbreviator::abbreviate(
                black_box(name),
                black_box(20),
            );
            black_box(s);
        });
    });

    c.bench_function("log4j_abbreviate_2", |b| {
        b.iter(|| {
            let s = tracing_declarative::formatter::log4j::abbreviator::abbreviate(
                black_box(name),
                black_box(2),
            );
            black_box(s);
        });
    });
}

criterion_group!(benches, bench_lexer, bench_date_format, bench_abbreviator);
criterion_main!(benches);
