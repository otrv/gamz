pub(crate) mod font;
mod rgba;

pub use font::{MAX_FONT_FILE_BYTES, read_font};
pub use rgba::read_rgba;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetError {
    InvalidImage,
    InvalidFont,
}
