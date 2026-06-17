//! Hot reload example - demonstrates automatic config reload on file changes.
//!
//! This example requires the "hot-reload" feature:
//! `cargo run --example hot_reload --features hot-reload`
//!
//! Note: hot-reload is currently unstable — `tracing` does not support
//! re-initializing the global dispatcher. See docs/ROADMAP.md.

#[cfg(not(feature = "hot-reload"))]
fn main() {
    eprintln!("This example requires the 'hot-reload' feature.");
    eprintln!("Run: cargo run --example hot_reload --features hot-reload");
}

#[cfg(feature = "hot-reload")]
fn main() {
    use std::fs;

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

    let handle = tracing_declarative::hot_reload::ReloadHandle::new(config_path)
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
