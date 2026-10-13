#[cfg(feature = "benchmark")]
#[path = "benchmark/build.rs"]
mod selected;

fn main() -> std::io::Result<()> {
    #[cfg(feature = "benchmark")]
    selected::build()?;
    Ok(())
}
