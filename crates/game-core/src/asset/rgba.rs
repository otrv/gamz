use platform_api::render::{Image, InvalidImage};

pub fn read_rgba(bytes: &[u8]) -> Result<Image<'_>, InvalidImage> {
    if bytes.len() < 8 || &bytes[..4] != b"RGBA" {
        return Err(InvalidImage);
    }
    let width = u16::from_le_bytes([bytes[4], bytes[5]]);
    let height = u16::from_le_bytes([bytes[6], bytes[7]]);
    Image::new(width, height, &bytes[8..])
}
