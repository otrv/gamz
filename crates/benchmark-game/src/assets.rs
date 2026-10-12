use game_core::asset::{AssetError, AssetId, read_font, read_rgba};
use game_core::font::Font;
use platform_api::render::{TextureId, TextureUpload};

include!(concat!(env!("OUT_DIR"), "/assets.gen.rs"));

pub(super) struct Assets {
    images: [(AssetId, TextureId); 2],
    fonts: [(AssetId, Font); 1],
}

impl Assets {
    pub(super) fn initialize() -> Result<(Self, [TextureUpload<'static>; 2]), AssetError> {
        let pack = EmbeddedPack::from_bytes(PACK_BYTES)?;
        let sprite = read_rgba(pack.bytes(ids::SPRITE)?)?;
        let atlas = read_rgba(pack.bytes(ids::FONT_ATLAS)?)?;
        let sprite_texture = TextureId::new(0);
        let atlas_texture = TextureId::new(1);
        let font = read_font(pack.bytes(ids::FONT)?, atlas_texture, atlas)?;
        Ok((
            Self {
                images: [
                    (ids::SPRITE, sprite_texture),
                    (ids::FONT_ATLAS, atlas_texture),
                ],
                fonts: [(ids::FONT, font)],
            },
            [
                TextureUpload {
                    id: sprite_texture,
                    image: sprite,
                },
                TextureUpload {
                    id: atlas_texture,
                    image: atlas,
                },
            ],
        ))
    }

    pub(super) fn image(&self, id: AssetId) -> TextureId {
        self.images
            .iter()
            .find(|(asset, _)| *asset == id)
            .expect("image was installed at startup")
            .1
    }

    pub(super) fn font(&self, id: AssetId) -> &Font {
        &self
            .fonts
            .iter()
            .find(|(asset, _)| *asset == id)
            .expect("font was installed at startup")
            .1
    }
}
