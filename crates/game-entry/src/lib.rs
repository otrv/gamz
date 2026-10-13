#![no_std]

#[cfg(all(feature = "game", feature = "example-benchmark"))]
compile_error!("select only one game; use --no-default-features for an example");

#[cfg(not(any(feature = "game", feature = "example-benchmark")))]
compile_error!("select the default game or one example feature");

#[cfg(feature = "game")]
pub use game::GameState as SelectedGame;

#[cfg(all(feature = "example-benchmark", not(feature = "game")))]
pub use examples::GameState as SelectedGame;
