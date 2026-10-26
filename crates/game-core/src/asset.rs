mod rgba;

pub use rgba::{InvalidImage, read_rgba};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackError {
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
    records: &'a [[u8; 28]],
    names: &'a [u8],
}

impl<'a, const MAX_ASSETS: usize, const MAX_BYTES: usize> Pack<'a, MAX_ASSETS, MAX_BYTES> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, PackError> {
        if bytes.len() > MAX_BYTES {
            return Err(PackError::BudgetExceeded);
        }
        if bytes.len() < 16 || &bytes[..4] != b"GPAK" || word(&bytes[4..8]) != 3 {
            return Err(PackError::InvalidPack);
        }
        let count = usize::try_from(word(&bytes[8..12])).map_err(|_| PackError::BudgetExceeded)?;
        if count > MAX_ASSETS {
            return Err(PackError::BudgetExceeded);
        }
        let end = count
            .checked_mul(28)
            .and_then(|length| length.checked_add(16))
            .ok_or(PackError::InvalidPack)?;
        let catalog = bytes.get(16..end).ok_or(PackError::InvalidPack)?;
        let table_length =
            usize::try_from(word(&bytes[12..16])).map_err(|_| PackError::InvalidPack)?;
        let payload_start = end
            .checked_add(table_length)
            .ok_or(PackError::InvalidPack)?;
        let names = bytes
            .get(end..payload_start)
            .ok_or(PackError::InvalidPack)?;
        let records = catalog.as_chunks::<28>().0;
        let mut previous = None;
        let mut previous_name = None;
        let mut name_end = 0;
        let mut payload_end = payload_start;
        for record in records {
            let id = word(&record[..4]);
            if previous.is_some_and(|value| value >= id) {
                return Err(PackError::InvalidPack);
            }
            let name_offset =
                usize::try_from(word(&record[4..8])).map_err(|_| PackError::InvalidPack)?;
            let name_length =
                usize::try_from(word(&record[8..12])).map_err(|_| PackError::InvalidPack)?;
            if name_offset != name_end || name_length == 0 {
                return Err(PackError::InvalidPack);
            }
            name_end = name_offset
                .checked_add(name_length)
                .ok_or(PackError::InvalidPack)?;
            let name = names
                .get(name_offset..name_end)
                .ok_or(PackError::InvalidPack)?;
            let name = core::str::from_utf8(name).map_err(|_| PackError::InvalidPack)?;
            if previous_name.is_some_and(|value| value >= name) {
                return Err(PackError::InvalidPack);
            }
            let offset = address(&record[12..20])?;
            let length = address(&record[20..28])?;
            if offset != payload_end {
                return Err(PackError::InvalidPack);
            }
            payload_end = offset.checked_add(length).ok_or(PackError::InvalidPack)?;
            if payload_end > bytes.len() {
                return Err(PackError::InvalidPack);
            }
            previous = Some(id);
            previous_name = Some(name);
        }
        if payload_end != bytes.len() || name_end != names.len() {
            return Err(PackError::InvalidPack);
        }
        Ok(Self {
            bytes,
            records,
            names,
        })
    }

    pub fn bytes(&self, id: AssetId) -> Result<&'a [u8], PackError> {
        let index = self
            .records
            .binary_search_by_key(&id.0, |record| word(&record[..4]))
            .map_err(|_| PackError::MissingAsset)?;
        Ok(self.payload(&self.records[index]))
    }

    pub fn bytes_by_name(&self, name: &str) -> Result<&'a [u8], PackError> {
        let index = self
            .records
            .binary_search_by(|record| {
                let offset = usize::try_from(word(&record[4..8])).expect("validated name offset");
                let length = usize::try_from(word(&record[8..12])).expect("validated name length");
                self.names[offset..offset + length].cmp(name.as_bytes())
            })
            .map_err(|_| PackError::MissingAsset)?;
        Ok(self.payload(&self.records[index]))
    }

    fn payload(&self, record: &[u8; 28]) -> &'a [u8] {
        let offset = address(&record[12..20]).expect("validated pack offset");
        let length = address(&record[20..28]).expect("validated pack length");
        &self.bytes[offset..offset + length]
    }
}

fn word(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("four-byte field"))
}

fn address(bytes: &[u8]) -> Result<usize, PackError> {
    usize::try_from(u64::from_le_bytes(
        bytes.try_into().expect("eight-byte field"),
    ))
    .map_err(|_| PackError::InvalidPack)
}
