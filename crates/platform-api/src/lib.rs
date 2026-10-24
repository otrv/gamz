#![no_std]

pub mod audio;
pub mod input;
pub mod memory;
pub mod render;
pub mod services;
pub mod timing;

mod game;
mod platform;

pub use game::Game;
pub use platform::Platform;
