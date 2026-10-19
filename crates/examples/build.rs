#[cfg(feature = "benchmark")]
#[path = "benchmark/build.rs"]
mod selected;

#[cfg(feature = "benchmark")]
fn main() -> std::io::Result<()> {
    selected::build()
}

#[cfg(not(feature = "benchmark"))]
fn main() {}
