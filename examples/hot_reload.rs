//! Hot reload example - demonstrates automatic config reload on file changes.
//!
//! This example requires the "hot-reload" feature (enabled by default).
//!
//! Run this example, then edit the tracing.toml file in /tmp to see
//! the logging configuration change in real-time.

use std::fs;

fn main() {
    let config_path = "/tmp/tracing-hot-reload.toml";

    let initial_config = r#"
[global]
level = "info"

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "default"
"#;

    fs::write(config_path, initial_config).expect("failed to write config");

    println!("Hot reload example");
    println!("====================");
    println!("Config file: {}", config_path);
    println!();
    println!("Initial config: level=info");
    println!();

    let handle = tracing_config::hot_reload::ReloadHandle::new(config_path)
        .expect("failed to create reload handle");

    handle.watch().expect("failed to start watching");
    handle.reload().expect("failed to perform initial reload");

    println!("Watching for config changes...");
    println!("Edit {} to change log level", config_path);
    println!("Press Ctrl+C to exit\n");

    tracing::info!("Info message (visible with info level)");
    tracing::debug!("Debug message (hidden with info level)");

    println!(
        "\nTry changing 'level = \"debug\"' in {} and save",
        config_path
    );
    println!(
        "Then run: echo '[global]\nlevel = \"debug\"' > {}",
        config_path
    );

    std::thread::sleep(std::time::Duration::from_secs(5));

    println!("\nExiting after 5 seconds");
}
