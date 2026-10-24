use platform_api::audio::{SampleRate, StereoFrame};
use platform_api::render;
use platform_api::render::{
    Color, GlyphQuad, MAX_TEXTURE_BYTES, MAX_TEXTURES, PixelRect, Rect, RectStyle, RenderCommand,
    Sampling, TextureUpload, Transform,
};
use serde::{Serialize, Serializer};

pub(super) struct AudioOutput<'a> {
    pub(super) sample_rate: SampleRate,
    pub(super) frames: &'a [StereoFrame],
}

impl Serialize for AudioOutput<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Fields<'a> {
            sample_rate: u32,
            frames: Samples<'a>,
        }

        Fields {
            sample_rate: self.sample_rate.hz(),
            frames: Samples(self.frames),
        }
        .serialize(serializer)
    }
}

struct Samples<'a>(&'a [StereoFrame]);

impl Serialize for Samples<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(|frame| [frame.left, frame.right]))
    }
}

#[derive(Serialize)]
pub(super) struct Startup<'a> {
    pub(super) uploads: Uploads<'a>,
}

pub(super) struct Uploads<'a>(pub(super) &'a [TextureUpload<'a>]);

impl Uploads<'_> {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for Uploads<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Upload<'a> {
            id: usize,
            image: Image<'a>,
        }
        #[derive(Serialize)]
        struct Image<'a> {
            width: u16,
            height: u16,
            rgba: &'a [u8],
        }

        assert!(self.0.len() <= MAX_TEXTURES);
        let mut bytes = 0;
        serializer.collect_seq(self.0.iter().map(|upload| {
            bytes += upload.image.rgba().len();
            assert!(bytes <= MAX_TEXTURE_BYTES);
            Upload {
                id: upload.id.index(),
                image: Image {
                    width: upload.image.width(),
                    height: upload.image.height(),
                    rgba: upload.image.rgba(),
                },
            }
        }))
    }
}

pub(super) struct FrameOutput<'a>(pub(super) &'a render::FrameOutput<'a>);

impl Serialize for FrameOutput<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Fields<'a> {
            canvas: [f32; 2],
            clear: [u8; 4],
            commands: Commands<'a>,
            #[serde(skip_serializing_if = "Uploads::is_empty")]
            uploads: Uploads<'a>,
        }

        Fields {
            canvas: [self.0.frame.canvas.width(), self.0.frame.canvas.height()],
            clear: color(self.0.frame.clear),
            commands: Commands(self.0.frame.commands()),
            uploads: Uploads(self.0.uploads()),
        }
        .serialize(serializer)
    }
}

struct Commands<'a>(&'a [RenderCommand<'a>]);

impl Serialize for Commands<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(Command::from))
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Command<'a> {
    SetCamera {
        origin: [f32; 2],
        zoom: f32,
    },
    SetClip {
        rect: Option<[f32; 4]>,
    },
    DrawSprite {
        texture: usize,
        source: [u16; 4],
        transform: TransformFields,
        tint: [u8; 4],
        sampling: &'static str,
    },
    DrawRect {
        rect: [f32; 4],
        style: Style,
    },
    DrawLine {
        start: [f32; 2],
        end: [f32; 2],
        width: f32,
        color: [u8; 4],
    },
    DrawText {
        glyphs: Glyphs<'a>,
        texture: usize,
        color: [u8; 4],
        sampling: &'static str,
    },
}

impl<'a> From<&RenderCommand<'a>> for Command<'a> {
    fn from(command: &RenderCommand<'a>) -> Self {
        match *command {
            RenderCommand::SetCamera(camera) => Self::SetCamera {
                origin: [camera.origin.x(), camera.origin.y()],
                zoom: camera.zoom.get(),
            },
            RenderCommand::SetClip(clip) => Self::SetClip {
                rect: clip.map(|clip| rect(clip.bounds())),
            },
            RenderCommand::DrawSprite(sprite) => Self::DrawSprite {
                texture: sprite.texture.index(),
                source: pixels(sprite.source),
                transform: sprite.transform.into(),
                tint: color(sprite.tint),
                sampling: sampling(sprite.sampling),
            },
            RenderCommand::DrawRect {
                rect: bounds,
                style,
            } => Self::DrawRect {
                rect: rect(bounds),
                style: match style {
                    RectStyle::Filled(value) => Style::Filled {
                        color: color(value),
                    },
                    RectStyle::Outline {
                        color: value,
                        width,
                    } => Style::Outline {
                        color: color(value),
                        width: width.get(),
                    },
                },
            },
            RenderCommand::DrawLine {
                start,
                end,
                width,
                color: value,
            } => Self::DrawLine {
                start: [start.x(), start.y()],
                end: [end.x(), end.y()],
                width: width.get(),
                color: color(value),
            },
            RenderCommand::DrawText(text) => Self::DrawText {
                glyphs: Glyphs(text.glyphs()),
                texture: text.texture.index(),
                color: color(text.color),
                sampling: sampling(text.sampling),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Style {
    Filled { color: [u8; 4] },
    Outline { color: [u8; 4], width: f32 },
}

#[derive(Serialize)]
struct TransformFields {
    center: [f32; 2],
    size: [f32; 2],
    rotation: f32,
}

impl From<Transform> for TransformFields {
    fn from(transform: Transform) -> Self {
        Self {
            center: [transform.center.x(), transform.center.y()],
            size: [transform.size.width(), transform.size.height()],
            rotation: transform.rotation.get(),
        }
    }
}

struct Glyphs<'a>(&'a [GlyphQuad]);

impl Serialize for Glyphs<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Glyph {
            source: [u16; 4],
            transform: TransformFields,
        }

        serializer.collect_seq(self.0.iter().map(|glyph| Glyph {
            source: pixels(glyph.source),
            transform: glyph.transform.into(),
        }))
    }
}

fn color(color: Color) -> [u8; 4] {
    [color.red, color.green, color.blue, color.alpha]
}

fn rect(rect: Rect) -> [f32; 4] {
    [
        rect.origin().x(),
        rect.origin().y(),
        rect.size().width(),
        rect.size().height(),
    ]
}

fn pixels(rect: PixelRect) -> [u16; 4] {
    [rect.x(), rect.y(), rect.width(), rect.height()]
}

fn sampling(sampling: Sampling) -> &'static str {
    match sampling {
        Sampling::Nearest => "nearest",
        Sampling::Linear => "linear",
    }
}
