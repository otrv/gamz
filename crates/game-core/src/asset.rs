use platform_api::render::Image;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetError {
    InvalidImage,
    InvalidFont,
}

pub fn read_rgba(bytes: &[u8]) -> Result<Image<'_>, AssetError> {
    if bytes.len() < 8 || &bytes[..4] != b"RGBA" {
        return Err(AssetError::InvalidImage);
    }
    let width = u16::from_le_bytes([bytes[4], bytes[5]]);
    let height = u16::from_le_bytes([bytes[6], bytes[7]]);
    Image::new(width, height, &bytes[8..]).map_err(|_| AssetError::InvalidImage)
}
