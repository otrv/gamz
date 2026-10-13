use std::env;
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn build() -> io::Result<()> {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=benchmark/build.rs");
    println!("cargo::rerun-if-changed=benchmark/assets");
    let output = PathBuf::from(
        env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("Cargo OUT_DIR is missing"))?,
    );
    asset_packer::build(Path::new("benchmark/assets"), &output, 16, 1024 * 1024)
}
