use platform_api::render::{TextureError, TextureId, TextureUpload};

use crate::limits::{MAX_TEXTURE_BYTES, MAX_TEXTURE_SIDE, MAX_TEXTURES};

pub(crate) struct Upload {
    pub(crate) id: TextureId,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

pub(crate) struct Textures {
    sizes: [usize; MAX_TEXTURES],
    bytes: usize,
    pending_bytes: usize,
    pub(crate) uploads: Vec<Upload>,
    pub(crate) releases: Vec<TextureId>,
}

impl Textures {
    pub(crate) fn new() -> Self {
        Self {
            sizes: [0; MAX_TEXTURES],
            bytes: 0,
            pending_bytes: 0,
            uploads: Vec::with_capacity(MAX_TEXTURES),
            releases: Vec::with_capacity(MAX_TEXTURES),
        }
    }

    pub(crate) fn upload(&mut self, upload: TextureUpload<'_>) -> Result<(), TextureError> {
        let id = usize::from(upload.id.0);
        if id >= MAX_TEXTURES {
            return Err(TextureError::InvalidId);
        }
        let length = u64::from(upload.width)
            .checked_mul(u64::from(upload.height))
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok());
        if upload.width == 0
            || upload.height == 0
            || upload.width > MAX_TEXTURE_SIDE
            || upload.height > MAX_TEXTURE_SIDE
            || length != Some(upload.rgba.len())
        {
            return Err(TextureError::InvalidImage);
        }
        let total = self.bytes - self.sizes[id] + upload.rgba.len();
        if total > MAX_TEXTURE_BYTES
            || self.pending_bytes + upload.rgba.len() > MAX_TEXTURE_BYTES
            || self.uploads.len() == MAX_TEXTURES
        {
            return Err(TextureError::CapacityExceeded);
        }
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(upload.rgba.len())
            .map_err(|_| TextureError::CapacityExceeded)?;
        rgba.extend_from_slice(upload.rgba);
        self.releases.retain(|released| *released != upload.id);
        self.uploads.push(Upload {
            id: upload.id,
            width: upload.width,
            height: upload.height,
            rgba,
        });
        self.bytes = total;
        self.pending_bytes += upload.rgba.len();
        self.sizes[id] = upload.rgba.len();
        Ok(())
    }

    pub(crate) fn release(&mut self, id: TextureId) -> Result<(), TextureError> {
        let index = usize::from(id.0);
        if index >= MAX_TEXTURES || self.sizes[index] == 0 {
            return Err(TextureError::InvalidId);
        }
        if self.releases.len() == MAX_TEXTURES {
            return Err(TextureError::CapacityExceeded);
        }
        self.bytes -= self.sizes[index];
        self.sizes[index] = 0;
        self.uploads.retain(|upload| upload.id != id);
        self.pending_bytes = self.uploads.iter().map(|upload| upload.rgba.len()).sum();
        self.releases.push(id);
        Ok(())
    }

    pub(crate) fn clear_pending(&mut self) {
        self.uploads.clear();
        self.releases.clear();
        self.pending_bytes = 0;
    }
}
