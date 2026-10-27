use math::{Rect, Vec4};
use platform_api::audio::StereoSample;
use platform_api::render;
use platform_api::render::{
    PixelRect, RectStyle, RenderCommand, Sampling, TextureId, TexturedQuad,
};
use serde::{Serialize, Serializer};

use crate::textures::Upload;

pub(super) struct AudioOutput<'a> {
    pub(super) sample_rate: u32,
    pub(super) frames: &'a [StereoSample],
}

impl Serialize for AudioOutput<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Fields<'a> {
            sample_rate: u32,
            frames: Samples<'a>,
        }

        Fields {
            sample_rate: self.sample_rate,
            frames: Samples(self.frames),
        }
        .serialize(serializer)
    }
}

struct Samples<'a>(&'a [StereoSample]);

impl Serialize for Samples<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(|frame| [frame.left, frame.right]))
    }
}

#[derive(Serialize)]
pub(super) struct Startup<'a> {
    pub(super) uploads: Uploads<'a>,
}

pub(super) struct Uploads<'a>(pub(super) &'a [Upload]);

impl Uploads<'_> {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for Uploads<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct UploadFields<'a> {
            id: u16,
            image: Image<'a>,
        }
        #[derive(Serialize)]
        struct Image<'a> {
            width: u32,
            height: u32,
            rgba: &'a [u8],
        }

        serializer.collect_seq(self.0.iter().map(|upload| UploadFields {
            id: upload.id.0,
            image: Image {
                width: upload.width,
                height: upload.height,
                rgba: &upload.rgba,
            },
        }))
    }
}

pub(super) struct FrameOutput<'a> {
    pub(super) frame: &'a render::Frame<'a>,
    pub(super) uploads: Uploads<'a>,
    pub(super) releases: &'a [TextureId],
}

impl Serialize for FrameOutput<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Fields<'a> {
            canvas: [f32; 2],
            clear: [f32; 4],
            commands: Commands<'a>,
            #[serde(skip_serializing_if = "Uploads::is_empty")]
            uploads: Uploads<'a>,
            #[serde(skip_serializing_if = "Releases::is_empty")]
            releases: Releases<'a>,
        }

        Fields {
            canvas: [self.frame.canvas_size.x, self.frame.canvas_size.y],
            clear: color(self.frame.clear_color),
            commands: Commands(self.frame.commands),
            uploads: Uploads(self.uploads.0),
            releases: Releases(self.releases),
        }
        .serialize(serializer)
    }
}

struct Releases<'a>(&'a [TextureId]);

impl Releases<'_> {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for Releases<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(|id| id.0))
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
    DrawRect {
        rect: [f32; 4],
        style: Style,
    },
    DrawLine {
        start: [f32; 2],
        end: [f32; 2],
        width: f32,
        color: [f32; 4],
    },
    DrawQuads {
        texture: u16,
        quads: Quads<'a>,
        tint: [f32; 4],
        sampling: &'static str,
    },
}

impl<'a> From<&RenderCommand<'a>> for Command<'a> {
    fn from(command: &RenderCommand<'a>) -> Self {
        match *command {
            RenderCommand::SetCamera { origin, zoom } => Self::SetCamera {
                origin: [origin.x, origin.y],
                zoom,
            },
            RenderCommand::SetClip(clip) => Self::SetClip {
                rect: clip.map(rect),
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
                        width,
                    },
                },
            },
            RenderCommand::DrawLine {
                start,
                end,
                width,
                color: value,
            } => Self::DrawLine {
                start: [start.x, start.y],
                end: [end.x, end.y],
                width,
                color: color(value),
            },
            RenderCommand::DrawQuads {
                texture,
                quads,
                tint,
                sampling: mode,
            } => Self::DrawQuads {
                texture: texture.0,
                quads: Quads(quads),
                tint: color(tint),
                sampling: sampling(mode),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Style {
    Filled { color: [f32; 4] },
    Outline { color: [f32; 4], width: f32 },
}

struct Quads<'a>(&'a [TexturedQuad]);

impl Serialize for Quads<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Quad {
            source: [u16; 4],
            center: [f32; 2],
            size: [f32; 2],
            rotation: f32,
        }

        serializer.collect_seq(self.0.iter().map(|quad| Quad {
            source: pixels(quad.source),
            center: [quad.center.x, quad.center.y],
            size: [quad.size.x, quad.size.y],
            rotation: quad.rotation.get(),
        }))
    }
}

fn color(color: Vec4) -> [f32; 4] {
    [color.x, color.y, color.z, color.w]
}

fn rect(rect: Rect) -> [f32; 4] {
    [rect.origin.x, rect.origin.y, rect.size.x, rect.size.y]
}

fn pixels(rect: PixelRect) -> [u16; 4] {
    [rect.x, rect.y, rect.width, rect.height]
}

fn sampling(sampling: Sampling) -> &'static str {
    match sampling {
        Sampling::Nearest => "nearest",
        Sampling::Linear => "linear",
    }
}
