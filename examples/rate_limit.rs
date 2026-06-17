//! Rate limiting example - demonstrates sampling/rate limiting configuration.
//!
//! This example shows how to configure rate limiting for high-volume logging scenarios.

fn main() {
    let config = r#"
[global]
level = "debug"

[sampling]
enabled = true
rate_per_second = 10

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "logback"
pattern = "%d{%H:%M:%S} [%level] %msg%n"
"#;

    println!("Rate limiting example - limiting to 10 logs/second");
    println!("Watch for dropped messages when exceeding the rate.\n");

    tracing_declarative::init_from_str(config).expect("failed to init tracing");

    println!("Attempting to log 20 messages rapidly...");
    for i in 0..20 {
        tracing::info!("message #{}", i);
    }

    println!("\nCheck stdout - approximately half should appear (rate limited to 10/sec)");

    std::thread::sleep(std::time::Duration::from_millis(1500));
    println!("\nAfter 1.5s delay, logging 5 more messages...");
    for i in 20..25 {
        tracing::info!("message #{}", i);
    }
}
