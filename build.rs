use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest_path = out_dir.join("version.rs");

    fs::write(
        &dest_path,
        format!(
            "pub const CRATE_NAME: &str = \"{}\";\npub const CRATE_VERSION: &str = \"{}\";\n",
            name, version
        ),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=Cargo.toml");
}
