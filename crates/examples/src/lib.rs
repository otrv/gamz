#![no_std]

#[cfg(not(feature = "benchmark"))]
compile_error!("select one example feature");

#[cfg(feature = "benchmark")]
#[path = "../benchmark/mod.rs"]
mod selected;

#[cfg(feature = "benchmark")]
pub use selected::GameState;
