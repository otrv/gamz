const LETTERS: [[u8; 7]; 26] = [
    [14, 17, 17, 31, 17, 17, 17],
    [30, 17, 17, 30, 17, 17, 30],
    [14, 17, 16, 16, 16, 17, 14],
    [30, 17, 17, 17, 17, 17, 30],
    [31, 16, 16, 30, 16, 16, 31],
    [31, 16, 16, 30, 16, 16, 16],
    [14, 17, 16, 23, 17, 17, 15],
    [17, 17, 17, 31, 17, 17, 17],
    [14, 4, 4, 4, 4, 4, 14],
    [7, 2, 2, 2, 18, 18, 12],
    [17, 18, 20, 24, 20, 18, 17],
    [16, 16, 16, 16, 16, 16, 31],
    [17, 27, 21, 21, 17, 17, 17],
    [17, 25, 21, 19, 17, 17, 17],
    [14, 17, 17, 17, 17, 17, 14],
    [30, 17, 17, 30, 16, 16, 16],
    [14, 17, 17, 17, 21, 18, 13],
    [30, 17, 17, 30, 20, 18, 17],
    [15, 16, 16, 14, 1, 1, 30],
    [31, 4, 4, 4, 4, 4, 4],
    [17, 17, 17, 17, 17, 17, 14],
    [17, 17, 17, 17, 17, 10, 4],
    [17, 17, 17, 21, 21, 21, 10],
    [17, 17, 10, 4, 10, 17, 17],
    [17, 17, 10, 4, 4, 4, 4],
    [31, 1, 2, 4, 8, 16, 31],
];

const fn rows(ascii: usize) -> [u8; 7] {
    match ascii {
        65..=90 => LETTERS[ascii - 65],
        97..=122 => LETTERS[ascii - 97],
        32 => [0; 7],
        47 => [1, 1, 2, 4, 8, 16, 16],
        49 => [4, 12, 4, 4, 4, 4, 14],
        50 => [14, 17, 1, 2, 4, 8, 31],
        53 => [31, 16, 16, 30, 1, 17, 14],
        _ => [14, 17, 1, 2, 4, 0, 4],
    }
}

const fn header<const N: usize>(width: u16, height: u16) -> [u8; N] {
    let mut bytes = [0; N];
    bytes[0] = b'R';
    bytes[1] = b'G';
    bytes[2] = b'B';
    bytes[3] = b'A';
    let w = width.to_le_bytes();
    let h = height.to_le_bytes();
    bytes[4] = w[0];
    bytes[5] = w[1];
    bytes[6] = h[0];
    bytes[7] = h[1];
    bytes
}

const fn font_atlas() -> [u8; 128 * 48 * 4 + 8] {
    let mut bytes = header::<{ 128 * 48 * 4 + 8 }>(128, 48);
    let mut glyph = 0;
    while glyph < 95 {
        let pattern = rows(glyph + 32);
        let mut y = 0;
        while y < 7 {
            let mut x = 0;
            while x < 5 {
                if pattern[y] & (1 << (4 - x)) != 0 {
                    let pixel = 8 + ((glyph / 16 * 8 + y) * 128 + glyph % 16 * 8 + x) * 4;
                    bytes[pixel] = 255;
                    bytes[pixel + 1] = 255;
                    bytes[pixel + 2] = 255;
                    bytes[pixel + 3] = 255;
                }
                x += 1;
            }
            y += 1;
        }
        glyph += 1;
    }
    bytes
}

const fn font_file() -> [u8; 1340] {
    let mut bytes = [0; 1340];
    bytes[0] = b'G';
    bytes[1] = b'F';
    bytes[2] = b'N';
    bytes[3] = b'1';
    bytes[4] = 8;
    let mut glyph = 0_u8;
    while glyph < 95 {
        let offset = 10 + glyph as usize * 14;
        bytes[offset] = glyph % 16 * 8;
        bytes[offset + 2] = glyph / 16 * 8;
        bytes[offset + 4] = if glyph == 0 { 0 } else { 5 };
        bytes[offset + 6] = if glyph == 0 { 0 } else { 7 };
        bytes[offset + 12] = 128;
        bytes[offset + 13] = 1;
        glyph += 1;
    }
    bytes
}

const fn sprite() -> [u8; 1032] {
    let mut bytes = header::<1032>(16, 16);
    let mut y = 0_usize;
    while y < 16 {
        let mut x = 0_usize;
        while x < 16 {
            let dx = (x * 2).abs_diff(15);
            let dy = (y * 2).abs_diff(15);
            let distance = dx * dx + dy * dy;
            if distance <= 210 {
                let pixel = 8 + (y * 16 + x) * 4;
                let shade = if distance > 130 || (x > 7 && y < 8) {
                    160
                } else {
                    255
                };
                bytes[pixel] = shade;
                bytes[pixel + 1] = shade;
                bytes[pixel + 2] = shade;
                bytes[pixel + 3] = if distance > 180 { 128 } else { 255 };
            }
            x += 1;
        }
        y += 1;
    }
    bytes
}

pub(super) static FONT_ATLAS: [u8; 24584] = font_atlas();
pub(super) static FONT_FILE: [u8; 1340] = font_file();
pub(super) static SPRITE: [u8; 1032] = sprite();
