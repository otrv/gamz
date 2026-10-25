use crate::font::{Font, FontError, LayoutError, MAX_TEXT_BYTES, TextMetrics};
use math::Radians;
use platform_api::render::{GlyphQuad, Image, PixelRect, Point, Scale, Size, Transform};

fn fixture() -> [u8; 1344] {
    let mut bytes = [0; 1344];
    bytes[..4].copy_from_slice(b"GFN1");
    bytes[4..6].copy_from_slice(&12_u16.to_le_bytes());
    bytes[8..10].copy_from_slice(&1_u16.to_le_bytes());
    for (character, words) in [
        (b' ', [0, 0, 0, 0, 0, 0, 384]),
        (b'A', [0, 0, 2, 3, -1, -2, 256]),
        (b'B', [2, 0, 1, 2, 1, 1, 320]),
        (b'?', [3, 0, 1, 1, 0, 0, 192]),
    ] {
        let start = 10 + usize::from(character - b' ') * 14;
        let indexed_words = words.into_iter().enumerate();
        for (index, word) in indexed_words {
            bytes[start + index * 2..start + index * 2 + 2]
                .copy_from_slice(&i16::to_le_bytes(word));
        }
    }
    let pair = u16::from(b'A' - b' ') * 95 + u16::from(b'B' - b' ');
    bytes[1340..1342].copy_from_slice(&pair.to_le_bytes());
    bytes[1342..1344].copy_from_slice(&(-64_i16).to_le_bytes());
    bytes
}

fn atlas() -> Image<'static> {
    Image::new(4, 4, &[255; 64]).unwrap()
}

fn empty_quad() -> GlyphQuad {
    GlyphQuad {
        source: PixelRect::new(0, 0, 0, 0),
        transform: Transform {
            center: Point::new(0.0, 0.0),
            size: Size::new(0.0, 0.0),
            rotation: Radians::new(0.0),
        },
    }
}

#[test]
fn decodes_fonts_independently_of_packs() {
    let mut bytes = [0; 1340];
    bytes[..4].copy_from_slice(b"GFN1");
    bytes[4] = 9;
    bytes[22..24].copy_from_slice(&384_u16.to_le_bytes());
    let font = Font::try_new(&bytes, atlas()).unwrap();
    assert_eq!(
        font.measure("  ", Scale::new(2.0)),
        Ok(TextMetrics {
            advance_size: Size::new(24.0, 18.0),
            glyph_count: 0,
        })
    );
    assert_eq!(
        Font::try_new(b"RGBA\x02\0\x01\0", atlas()).err(),
        Some(FontError::InvalidData)
    );
    bytes[4] = 0;
    assert_eq!(
        Font::try_new(&bytes, atlas()).err(),
        Some(FontError::InvalidData)
    );
}

#[test]
fn rejects_truncated_and_invalid_font_records() {
    let bytes = fixture();
    for length in 0..bytes.len() {
        assert_eq!(
            Font::try_new(&bytes[..length], atlas()).err(),
            Some(FontError::InvalidData),
            "truncation at {length}"
        );
    }
    for (offset, value) in [
        (0, 0),
        (4, 1025),
        (6, 1),
        (8, 1025),
        (472, 3),
        (474, 2),
        (480, -1025),
        (482, 1025),
        (1340, 9025),
    ] {
        let mut invalid = bytes;
        invalid[offset..offset + 2].copy_from_slice(&i16::to_le_bytes(value));
        assert_eq!(
            Font::try_new(&invalid, atlas()).err(),
            Some(FontError::InvalidData),
            "field at {offset} with value {value}"
        );
    }
}

#[test]
fn prepares_kerned_multiline_glyphs_with_asymmetric_offsets() {
    let font = Font::try_new(&fixture(), atlas()).unwrap();
    let scale = Scale::new(2.0);
    let origin = Point::new(10.0, 20.0);
    let mut output = [empty_quad(); 3];
    let prepared = font.prepare("AB\nA", origin, scale, &mut output).unwrap();
    assert_eq!(
        prepared.metrics,
        TextMetrics {
            advance_size: Size::new(16.0, 48.0),
            glyph_count: 3,
        }
    );
    assert_eq!(font.measure("AB\nA", scale), Ok(prepared.metrics));
    assert_eq!(prepared.glyphs.len(), 3);
    let expected = [
        (
            PixelRect::new(0, 0, 2, 3),
            Point::new(10.0, 19.0),
            Size::new(4.0, 6.0),
        ),
        (
            PixelRect::new(2, 0, 1, 2),
            Point::new(19.0, 24.0),
            Size::new(2.0, 4.0),
        ),
        (
            PixelRect::new(0, 0, 2, 3),
            Point::new(10.0, 43.0),
            Size::new(4.0, 6.0),
        ),
    ];
    let comparisons = prepared.glyphs.iter().zip(expected);
    for (glyph, (source, center, size)) in comparisons {
        assert_eq!(glyph.source, source);
        assert_eq!(glyph.transform.center, center);
        assert_eq!(glyph.transform.size, size);
        assert_eq!(glyph.transform.rotation, Radians::new(0.0));
    }
    let prepared = font.prepare("A\nB", origin, scale, &mut output).unwrap();
    assert_eq!(prepared.glyphs.len(), 2);
    assert_eq!(prepared.metrics.advance_size, Size::new(10.0, 48.0));
    assert_eq!(prepared.glyphs[1].transform.center, Point::new(13.0, 48.0));
    assert_eq!(
        font.prepare("AB\nA", origin, scale, &mut output[..2]).err(),
        Some(LayoutError::OutputTooSmall)
    );
}

#[test]
fn fallback_and_empty_glyphs_preserve_advance_without_extra_quads() {
    let font = Font::try_new(&fixture(), atlas()).unwrap();
    let origin = Point::new(0.0, 0.0);
    let scale = Scale::new(1.0);
    let mut output = [empty_quad(); 2];
    let prepared = font.prepare("é\t", origin, scale, &mut output).unwrap();
    assert_eq!(
        prepared.metrics,
        TextMetrics {
            advance_size: Size::new(6.0, 12.0),
            glyph_count: 2,
        }
    );
    assert_eq!(font.measure("é\t", scale), Ok(prepared.metrics));
    assert_eq!(prepared.glyphs.len(), 2);
    let comparisons = prepared.glyphs.iter().zip([0.5, 3.5]);
    for (glyph, x) in comparisons {
        assert_eq!(glyph.source, PixelRect::new(3, 0, 1, 1));
        assert_eq!(glyph.transform.center, Point::new(x, 0.5));
    }
    let prepared = font.prepare("A B", origin, scale, &mut output).unwrap();
    assert_eq!(prepared.metrics.advance_size, Size::new(15.0, 12.0));
    assert_eq!(prepared.metrics.glyph_count, 2);
    assert_eq!(prepared.glyphs[1].transform.center, Point::new(11.5, 2.0));
    for (text, size) in [
        ("", Size::new(0.0, 0.0)),
        ("  ", Size::new(12.0, 12.0)),
        ("\n", Size::new(0.0, 24.0)),
    ] {
        let prepared = font.prepare(text, origin, scale, &mut []).unwrap();
        assert!(prepared.glyphs.is_empty());
        assert_eq!(
            prepared.metrics,
            TextMetrics {
                advance_size: size,
                glyph_count: 0
            }
        );
        assert_eq!(font.measure(text, scale), Ok(prepared.metrics));
    }
}

#[test]
fn text_limit_counts_bytes_and_accepts_exact_capacity() {
    let font = Font::try_new(&fixture(), atlas()).unwrap();
    let origin = Point::new(0.0, 0.0);
    let scale = Scale::new(1.0);
    let mut bytes = [b' '; MAX_TEXT_BYTES + 1];
    let exact = core::str::from_utf8(&bytes[..MAX_TEXT_BYTES]).unwrap();
    let metrics = TextMetrics {
        advance_size: Size::new(24_576.0, 12.0),
        glyph_count: 0,
    };
    assert_eq!(font.measure(exact, scale), Ok(metrics));
    assert_eq!(
        font.prepare(exact, origin, scale, &mut []).unwrap().metrics,
        metrics
    );
    for prefix in [*b"  ", *b"\xc3\xa9"] {
        bytes[..2].copy_from_slice(&prefix);
        let overlong = core::str::from_utf8(&bytes).unwrap();
        assert_eq!(font.measure(overlong, scale), Err(LayoutError::TextTooLong));
        assert_eq!(
            font.prepare(overlong, origin, scale, &mut []).err(),
            Some(LayoutError::TextTooLong)
        );
    }
}

#[test]
fn reports_coordinate_overflow_beyond_float_rounding() {
    let mut bytes = fixture();
    bytes[4..6].copy_from_slice(&1024_u16.to_le_bytes());
    bytes[484..486].copy_from_slice(&u16::MAX.to_le_bytes());
    let font = Font::try_new(&bytes, atlas()).unwrap();
    let scale = Scale::new(32.0);
    let mut output = [empty_quad(); 3];
    for (text, origin) in [
        ("AAA", Point::new(1.0e12, 0.0)),
        ("A\n\nA", Point::new(0.0, 1.0e12)),
    ] {
        assert!(font.measure(text, scale).is_ok());
        assert!(
            font.prepare(text, Point::new(0.0, 0.0), scale, &mut output)
                .is_ok()
        );
        assert_eq!(
            font.prepare(text, origin, scale, &mut output).err(),
            Some(LayoutError::OutOfBounds)
        );
    }
}
