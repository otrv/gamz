#![no_std]

#[cfg(all(feature = "benchmark", feature = "breakout"))]
compile_error!("select only one example feature");

#[cfg(not(any(feature = "benchmark", feature = "breakout")))]
compile_error!("select one example feature");

#[cfg(feature = "benchmark")]
#[path = "../benchmark/mod.rs"]
mod selected;

#[cfg(all(feature = "breakout", not(feature = "benchmark")))]
#[path = "../breakout/mod.rs"]
mod selected;

#[cfg(any(feature = "benchmark", feature = "breakout"))]
pub use selected::GameState;
