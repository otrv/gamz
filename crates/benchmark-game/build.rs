use std::env;
use std::io;
use std::path::{Path, PathBuf};

fn main() -> io::Result<()> {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=assets");
    let output = PathBuf::from(
        env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("Cargo OUT_DIR is missing"))?,
    );
    asset_packer::build(Path::new("assets"), &output, 16, 1024 * 1024)
}
