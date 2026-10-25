use crate::asset::{AssetId, Pack, PackError, read_rgba};
use platform_api::render::InvalidImage;

fn fixture() -> [u8; 84] {
    let mut bytes = [0; 84];
    bytes[..16].copy_from_slice(b"GPAK\x02\0\0\0\x03\0\0\0\0\0\0\0");
    bytes[16..36].copy_from_slice(&[7, 0, 0, 0, 76, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0]);
    bytes[36..56].copy_from_slice(&[
        232, 3, 0, 0, 79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    bytes[56..76].copy_from_slice(&[
        0, 40, 107, 238, 79, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0,
    ]);
    bytes[76..].copy_from_slice(b"\xff\0\x7fhello");
    bytes
}

#[test]
fn resolves_ids_to_opaque_bytes_including_empty_payloads() {
    let bytes = fixture();
    let last = {
        let pack = Pack::<3, 84>::from_bytes(&bytes).unwrap();
        assert_eq!(pack.bytes(AssetId::new(1000)).unwrap(), b"");
        assert_eq!(pack.bytes(AssetId::new(7)).unwrap(), b"\xff\0\x7f");
        pack.bytes(AssetId::new(4_000_000_000)).unwrap()
    };
    assert_eq!(last, b"hello");
}

#[test]
fn reports_missing_ids() {
    let bytes = fixture();
    let pack = Pack::<3, 84>::from_bytes(&bytes).unwrap();
    for id in [0, 8, 999, 1001, u32::MAX] {
        assert_eq!(pack.bytes(AssetId::new(id)), Err(PackError::MissingAsset));
    }
}

#[test]
fn enforces_game_budgets_including_exact_boundaries() {
    let bytes = fixture();
    assert!(Pack::<3, 84>::from_bytes(&bytes).is_ok());
    assert_eq!(
        Pack::<2, 84>::from_bytes(&bytes).err(),
        Some(PackError::BudgetExceeded)
    );
    assert_eq!(
        Pack::<3, 83>::from_bytes(&bytes).err(),
        Some(PackError::BudgetExceeded)
    );
    let empty = b"GPAK\x02\0\0\0\0\0\0\0\0\0\0\0";
    assert!(Pack::<0, 16>::from_bytes(empty).is_ok());
}

#[test]
fn rejects_truncated_headers_catalogs_and_payloads() {
    let bytes = fixture();
    for length in 0..bytes.len() {
        assert_eq!(
            Pack::<3, 84>::from_bytes(&bytes[..length]).err(),
            Some(PackError::InvalidPack),
            "truncation at {length}"
        );
    }
}

#[test]
fn rejects_ambiguous_or_invalid_catalogs() {
    for (offset, value) in [(0, 0), (4, 1), (12, 1), (36, 7), (56, 6)] {
        let mut bytes = fixture();
        bytes[offset..offset + 4].copy_from_slice(&u32::to_le_bytes(value));
        assert_eq!(
            Pack::<3, 84>::from_bytes(&bytes).err(),
            Some(PackError::InvalidPack)
        );
    }
    for (offset, value) in [
        (20, 75),
        (20, 77),
        (28, 0),
        (28, u64::MAX),
        (40, u64::MAX),
        (48, 1),
        (68, 4),
    ] {
        let mut bytes = fixture();
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        assert_eq!(
            Pack::<3, 84>::from_bytes(&bytes).err(),
            Some(PackError::InvalidPack)
        );
    }
}

#[test]
fn decodes_images_independently_of_packs() {
    let image_bytes = b"RGBA\x02\0\x01\0\x0b\x16\x21\x2c\x01\x02\x03\x04";
    let image = read_rgba(image_bytes).unwrap();
    assert_eq!((image.width(), image.height()), (2, 1));
    assert_eq!(image.rgba(), &[11, 22, 33, 44, 1, 2, 3, 4]);
    assert_eq!(read_rgba(b"GFN1\0\0\0\0").err(), Some(InvalidImage));
    for length in 0..image_bytes.len() {
        assert_eq!(read_rgba(&image_bytes[..length]).err(), Some(InvalidImage));
    }
}
