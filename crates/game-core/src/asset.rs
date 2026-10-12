pub(crate) mod font;
mod rgba;

pub use font::{MAX_FONT_FILE_BYTES, read_font};
pub use rgba::read_rgba;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetError {
    InvalidImage,
    InvalidFont,
    InvalidPack,
    BudgetExceeded,
    MissingAsset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetId(u32);

impl AssetId {
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

pub struct Pack<'a, const MAX_ASSETS: usize, const MAX_BYTES: usize> {
    bytes: &'a [u8],
    records: &'a [[u8; 20]],
}

impl<'a, const MAX_ASSETS: usize, const MAX_BYTES: usize> Pack<'a, MAX_ASSETS, MAX_BYTES> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, AssetError> {
        if bytes.len() > MAX_BYTES {
            return Err(AssetError::BudgetExceeded);
        }
        if bytes.len() < 16
            || &bytes[..4] != b"GPAK"
            || word(&bytes[4..8]) != 2
            || word(&bytes[12..16]) != 0
        {
            return Err(AssetError::InvalidPack);
        }
        let count = usize::try_from(word(&bytes[8..12])).map_err(|_| AssetError::BudgetExceeded)?;
        if count > MAX_ASSETS {
            return Err(AssetError::BudgetExceeded);
        }
        let end = count
            .checked_mul(20)
            .and_then(|length| length.checked_add(16))
            .ok_or(AssetError::InvalidPack)?;
        let catalog = bytes.get(16..end).ok_or(AssetError::InvalidPack)?;
        let records = catalog.as_chunks::<20>().0;
        let mut previous = None;
        let mut payload_end = end;
        for record in records {
            let id = word(&record[..4]);
            if previous.is_some_and(|value| value >= id) {
                return Err(AssetError::InvalidPack);
            }
            let offset = address(&record[4..12])?;
            let length = address(&record[12..20])?;
            if offset != payload_end {
                return Err(AssetError::InvalidPack);
            }
            payload_end = offset.checked_add(length).ok_or(AssetError::InvalidPack)?;
            if payload_end > bytes.len() {
                return Err(AssetError::InvalidPack);
            }
            previous = Some(id);
        }
        if payload_end != bytes.len() {
            return Err(AssetError::InvalidPack);
        }
        Ok(Self { bytes, records })
    }

    pub fn bytes(&self, id: AssetId) -> Result<&'a [u8], AssetError> {
        let index = self
            .records
            .binary_search_by_key(&id.0, |record| word(&record[..4]))
            .map_err(|_| AssetError::MissingAsset)?;
        let record = &self.records[index];
        let offset = address(&record[4..12]).expect("validated pack offset");
        let length = address(&record[12..20]).expect("validated pack length");
        Ok(&self.bytes[offset..offset + length])
    }
}

fn word(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("four-byte field"))
}

fn address(bytes: &[u8]) -> Result<usize, AssetError> {
    usize::try_from(u64::from_le_bytes(
        bytes.try_into().expect("eight-byte field"),
    ))
    .map_err(|_| AssetError::InvalidPack)
}
