use crate::asset::{AssetId, InvalidImage, Pack, PackError, read_rgba};

fn fixture() -> [u8; 128] {
    let mut bytes = [0; 128];
    bytes[..16].copy_from_slice(b"GPAK\x03\0\0\0\x03\0\0\0\x14\0\0\0");
    bytes[16..44].copy_from_slice(&[
        7, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 120, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0,
    ]);
    bytes[44..72].copy_from_slice(&[
        232, 3, 0, 0, 5, 0, 0, 0, 9, 0, 0, 0, 123, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    bytes[72..100].copy_from_slice(&[
        0, 40, 107, 238, 14, 0, 0, 0, 6, 0, 0, 0, 123, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0,
    ]);
    bytes[100..120].copy_from_slice(b"a.binempty.bin\xc3\xa9.bin");
    bytes[120..].copy_from_slice(b"\xff\0\x7fhello");
    bytes
}

#[test]
fn resolves_ids_and_names_to_opaque_bytes_including_empty_payloads() {
    let bytes = fixture();
    let last = {
        let pack = Pack::<3, 128>::from_bytes(&bytes).unwrap();
        assert_eq!(pack.bytes(AssetId::new(1000)).unwrap(), b"");
        assert_eq!(pack.bytes(AssetId::new(7)).unwrap(), b"\xff\0\x7f");
        assert_eq!(pack.bytes(AssetId::new(4_000_000_000)).unwrap(), b"hello");
        assert_eq!(pack.bytes_by_name("empty.bin").unwrap(), b"");
        assert_eq!(pack.bytes_by_name("a.bin").unwrap(), b"\xff\0\x7f");
        pack.bytes_by_name("é.bin").unwrap()
    };
    assert_eq!(last, b"hello");
}

#[test]
fn reports_missing_ids_and_exact_names() {
    let bytes = fixture();
    let pack = Pack::<3, 128>::from_bytes(&bytes).unwrap();
    for id in [0, 8, 999, 1001, u32::MAX] {
        assert_eq!(pack.bytes(AssetId::new(id)), Err(PackError::MissingAsset));
    }
    for name in ["", "A.bin", "a", "a.bin/", "b.bin", "é", "ê.bin"] {
        assert_eq!(pack.bytes_by_name(name), Err(PackError::MissingAsset));
    }
}

#[test]
fn enforces_game_budgets_including_exact_boundaries() {
    let bytes = fixture();
    assert!(Pack::<3, 128>::from_bytes(&bytes).is_ok());
    assert_eq!(
        Pack::<2, 128>::from_bytes(&bytes).err(),
        Some(PackError::BudgetExceeded)
    );
    assert_eq!(
        Pack::<3, 127>::from_bytes(&bytes).err(),
        Some(PackError::BudgetExceeded)
    );
    let empty = b"GPAK\x03\0\0\0\0\0\0\0\0\0\0\0";
    let pack = Pack::<0, 16>::from_bytes(empty).unwrap();
    assert_eq!(pack.bytes_by_name("a.bin"), Err(PackError::MissingAsset));
}

#[test]
fn rejects_truncated_headers_catalogs_and_payloads() {
    let bytes = fixture();
    for length in 0..bytes.len() {
        assert_eq!(
            Pack::<3, 128>::from_bytes(&bytes[..length]).err(),
            Some(PackError::InvalidPack),
            "truncation at {length}"
        );
    }
}

#[test]
fn rejects_ambiguous_or_invalid_catalogs() {
    for (offset, value) in [
        (0, 0),
        (4, 2),
        (12, 19),
        (12, 21),
        (12, u32::MAX),
        (44, 7),
        (72, 6),
        (20, 1),
        (24, 0),
        (24, u32::MAX),
        (48, 4),
        (48, 6),
        (76, u32::MAX),
        (80, 5),
        (80, 7),
    ] {
        let mut bytes = fixture();
        bytes[offset..offset + 4].copy_from_slice(&u32::to_le_bytes(value));
        assert_eq!(
            Pack::<3, 128>::from_bytes(&bytes).err(),
            Some(PackError::InvalidPack)
        );
    }
    for (offset, value) in [
        (28, 119),
        (28, 121),
        (36, 0),
        (36, u64::MAX),
        (56, u64::MAX),
        (64, 1),
        (92, 4),
    ] {
        let mut bytes = fixture();
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        assert_eq!(
            Pack::<3, 128>::from_bytes(&bytes).err(),
            Some(PackError::InvalidPack)
        );
    }
}

#[test]
fn rejects_invalid_utf8_unsorted_and_duplicate_names() {
    for value in [0xff, b'z'] {
        let mut bytes = fixture();
        bytes[100] = value;
        assert_eq!(
            Pack::<3, 128>::from_bytes(&bytes).err(),
            Some(PackError::InvalidPack)
        );
    }
    let mut bytes = fixture();
    bytes[52..56].copy_from_slice(&5_u32.to_le_bytes());
    bytes[76..80].copy_from_slice(&10_u32.to_le_bytes());
    bytes[80..84].copy_from_slice(&10_u32.to_le_bytes());
    bytes[100..120].copy_from_slice(b"a.bina.binz-last.bin");
    assert_eq!(
        Pack::<3, 128>::from_bytes(&bytes).err(),
        Some(PackError::InvalidPack)
    );
}

#[test]
fn decodes_images_independently_of_packs() {
    let image_bytes = b"RGBA\x02\0\x01\0\x0b\x16\x21\x2c\x01\x02\x03\x04";
    let image = read_rgba(image_bytes).unwrap();
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(image.rgba, &[11, 22, 33, 44, 1, 2, 3, 4]);
    assert_eq!(read_rgba(b"GFN1\0\0\0\0").err(), Some(InvalidImage));
    for invalid in [
        &b"RGBA\0\0\x01\0"[..],
        &b"RGBA\x01\0\0\0"[..],
        &b"RGBA\xff\xff\xff\xff"[..],
        &b"RGBA\x01\0\x01\0\0\0\0\0\0"[..],
    ] {
        assert_eq!(read_rgba(invalid).err(), Some(InvalidImage));
    }
    for length in 0..image_bytes.len() {
        assert_eq!(read_rgba(&image_bytes[..length]).err(), Some(InvalidImage));
    }
}
