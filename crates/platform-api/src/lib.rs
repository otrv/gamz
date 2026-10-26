#![no_std]

pub mod audio;
pub mod file;
pub mod input;
pub mod memory;
pub mod render;
pub mod request;
pub mod window;

mod game;
mod platform;

pub use game::{FrameError, Game, StartupError};
pub use platform::Platform;
