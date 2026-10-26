use platform_api::render::Image;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidImage;

pub fn read_rgba(bytes: &[u8]) -> Result<Image<'_>, InvalidImage> {
    if bytes.len() < 8 || &bytes[..4] != b"RGBA" {
        return Err(InvalidImage);
    }
    let width = u16::from_le_bytes([bytes[4], bytes[5]]);
    let height = u16::from_le_bytes([bytes[6], bytes[7]]);
    let rgba = &bytes[8..];
    let length = usize::from(width)
        .checked_mul(usize::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(InvalidImage)?;
    if width == 0 || height == 0 || rgba.len() != length {
        return Err(InvalidImage);
    }
    Ok(Image {
        width,
        height,
        rgba,
    })
}
