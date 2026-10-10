use crate::{GPU_QUADS, RendererError, Texture, error};
use num_traits::ToPrimitive;
use platform_api::render::{
    Camera, Canvas, CanvasRect, Color, Frame, MAX_COMMANDS, MAX_TEXTURES, PixelRect, RectStyle,
    RenderCommand, Sampling, Sprite,
};

pub(super) const VERTEX_BYTES: usize = 48;

#[derive(Clone, Copy)]
pub(super) struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    canvas: Canvas,
}

impl Viewport {
    pub(crate) fn new(canvas: Canvas, width: u32, height: u32) -> Option<Self> {
        let width = f64::from(width);
        let height = f64::from(height);
        let scale = (width / f64::from(canvas.width())).min(height / f64::from(canvas.height()));
        let fitted_width = (f64::from(canvas.width()) * scale).min(width);
        let fitted_height = (f64::from(canvas.height()) * scale).min(height);
        if fitted_width < 1.0 || fitted_height < 1.0 {
            return None;
        }
        Some(Self {
            x: ((width - fitted_width) * 0.5).to_f32().unwrap(),
            y: ((height - fitted_height) * 0.5).to_f32().unwrap(),
            width: fitted_width.to_f32().unwrap(),
            height: fitted_height.to_f32().unwrap(),
            canvas,
        })
    }

    fn clip(self, rect: Option<CanvasRect>) -> [u32; 4] {
        let (left, top, right, bottom) = rect.map_or(
            (0.0, 0.0, self.canvas.width(), self.canvas.height()),
            |rect| {
                let rect = rect.bounds();
                (
                    rect.origin().x(),
                    rect.origin().y(),
                    rect.origin().x() + rect.size().width(),
                    rect.origin().y() + rect.size().height(),
                )
            },
        );
        let x = (self.x + left.clamp(0.0, self.canvas.width()) * self.width / self.canvas.width())
            .ceil()
            .to_u32()
            .unwrap();
        let y = (self.y
            + top.clamp(0.0, self.canvas.height()) * self.height / self.canvas.height())
        .ceil()
        .to_u32()
        .unwrap();
        let right = (self.x
            + right.clamp(0.0, self.canvas.width()) * self.width / self.canvas.width())
        .floor()
        .to_u32()
        .unwrap();
        let bottom = (self.y
            + bottom.clamp(0.0, self.canvas.height()) * self.height / self.canvas.height())
        .floor()
        .to_u32()
        .unwrap();
        [x, y, right.saturating_sub(x), bottom.saturating_sub(y)]
    }
}

pub(super) struct Batch {
    pub texture: Option<usize>,
    pub sampling: Sampling,
    pub clip: [u32; 4],
    pub start: u32,
    pub end: u32,
}

struct State {
    camera: Camera,
    viewport: Viewport,
    clip: [u32; 4],
}

pub(super) struct Geometry {
    pub bytes: Vec<u8>,
    pub batches: Vec<Batch>,
}

impl Geometry {
    pub(crate) fn new() -> Result<Self, RendererError> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(GPU_QUADS * 4 * VERTEX_BYTES)
            .map_err(error)?;
        let mut batches = Vec::new();
        batches.try_reserve_exact(GPU_QUADS).map_err(error)?;
        Ok(Self { bytes, batches })
    }

    pub(crate) fn build(
        &mut self,
        frame: &Frame<'_>,
        viewport: Viewport,
        textures: &[Option<Texture>; MAX_TEXTURES],
    ) {
        assert!(frame.commands.len() <= MAX_COMMANDS);
        self.bytes.clear();
        self.batches.clear();
        let mut state = State {
            camera: Camera::default(),
            viewport,
            clip: viewport.clip(None),
        };
        self.solid(
            &mut state,
            [0.0, 0.0, frame.canvas.width(), frame.canvas.height()],
            frame.clear,
        );
        for command in frame.commands {
            match *command {
                RenderCommand::SetCamera(camera) => {
                    state.camera = camera;
                }
                RenderCommand::SetClip(rect) => {
                    state.clip = viewport.clip(rect);
                }
                RenderCommand::DrawSprite(sprite) => self.sprite(&mut state, sprite, textures),
                RenderCommand::DrawRect { rect, style } => {
                    let x = rect.origin().x();
                    let y = rect.origin().y();
                    let w = rect.size().width();
                    let h = rect.size().height();
                    match style {
                        RectStyle::Filled(color) => self.solid(&mut state, [x, y, w, h], color),
                        RectStyle::Outline { color, width } => {
                            let stroke = width.get().min(w * 0.5).min(h * 0.5);
                            for bounds in [
                                [x, y, w, stroke],
                                [x, y + h - stroke, w, stroke],
                                [x, y + stroke, stroke, h - 2.0 * stroke],
                                [x + w - stroke, y + stroke, stroke, h - 2.0 * stroke],
                            ] {
                                self.solid(&mut state, bounds, color);
                            }
                        }
                    }
                }
                RenderCommand::DrawLine {
                    start,
                    end,
                    width,
                    color,
                } => {
                    let dx = end.x() - start.x();
                    let dy = end.y() - start.y();
                    let length = dx.hypot(dy);
                    if length > 0.0 {
                        let nx = -dy / length * width.get() * 0.5;
                        let ny = dx / length * width.get() * 0.5;
                        self.quad(
                            &mut state,
                            [
                                [start.x() + nx, start.y() + ny],
                                [end.x() + nx, end.y() + ny],
                                [end.x() - nx, end.y() - ny],
                                [start.x() - nx, start.y() - ny],
                            ],
                            None,
                            Sampling::Nearest,
                            [[0.0, 0.0, 1.0, 1.0], [0.5; 4]],
                            color,
                        );
                    }
                }
                RenderCommand::DrawText(text) => {
                    for glyph in text.glyphs() {
                        self.sprite(
                            &mut state,
                            Sprite {
                                texture: text.texture,
                                source: glyph.source,
                                transform: glyph.transform,
                                tint: text.color,
                                sampling: text.sampling,
                            },
                            textures,
                        );
                    }
                }
            }
        }
    }

    fn solid(&mut self, state: &mut State, [x, y, w, h]: [f32; 4], color: Color) {
        if w == 0.0 || h == 0.0 {
            return;
        }
        self.quad(
            state,
            [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
            None,
            Sampling::Nearest,
            [[0.0, 0.0, 1.0, 1.0], [0.5; 4]],
            color,
        );
    }

    fn sprite(
        &mut self,
        state: &mut State,
        sprite: Sprite,
        textures: &[Option<Texture>; MAX_TEXTURES],
    ) {
        let texture = textures[sprite.texture.index()]
            .as_ref()
            .expect("texture not uploaded");
        let source: PixelRect = sprite.source;
        assert!(u32::from(source.x()) + u32::from(source.width()) <= u32::from(texture.width));
        assert!(u32::from(source.y()) + u32::from(source.height()) <= u32::from(texture.height));
        if source.width() == 0
            || source.height() == 0
            || sprite.transform.size.width() == 0.0
            || sprite.transform.size.height() == 0.0
        {
            return;
        }
        let tw = f32::from(texture.width);
        let th = f32::from(texture.height);
        let uv = [
            f32::from(source.x()) / tw,
            f32::from(source.y()) / th,
            (f32::from(source.x()) + f32::from(source.width())) / tw,
            (f32::from(source.y()) + f32::from(source.height())) / th,
        ];
        let bounds = [
            uv[0] + 0.5 / tw,
            uv[1] + 0.5 / th,
            uv[2] - 0.5 / tw,
            uv[3] - 0.5 / th,
        ];
        let (sin, cos) = sprite.transform.rotation.get().sin_cos();
        let w = sprite.transform.size.width() * 0.5;
        let h = sprite.transform.size.height() * 0.5;
        let points = [[-w, -h], [w, -h], [w, h], [-w, h]].map(|[x, y]| {
            [
                sprite.transform.center.x() + x * cos - y * sin,
                sprite.transform.center.y() + x * sin + y * cos,
            ]
        });
        self.quad(
            state,
            points,
            Some(sprite.texture.index()),
            sprite.sampling,
            [uv, bounds],
            sprite.tint,
        );
    }

    fn quad(
        &mut self,
        state: &mut State,
        points: [[f32; 2]; 4],
        texture: Option<usize>,
        sampling: Sampling,
        [uv, bounds]: [[f32; 4]; 2],
        color: Color,
    ) {
        if state.clip[2] == 0 || state.clip[3] == 0 {
            return;
        }
        let index = self.bytes.len() / (VERTEX_BYTES * 4);
        assert!(index < GPU_QUADS, "quad budget exhausted");
        let index = u32::try_from(index).unwrap();
        let compatible = self.batches.last().is_some_and(|batch| {
            batch.texture == texture && batch.sampling == sampling && batch.clip == state.clip
        });
        if compatible {
            self.batches.last_mut().unwrap().end = index + 1;
        } else {
            assert!(self.batches.len() < GPU_QUADS);
            self.batches.push(Batch {
                texture,
                sampling,
                clip: state.clip,
                start: index,
                end: index + 1,
            });
        }
        let colors = [
            linear(color.0),
            linear(color.1),
            linear(color.2),
            f32::from(color.3) / 255.0,
        ];
        let points = points.map(|[x, y]| {
            let x = (f64::from(x) - f64::from(state.camera.origin.x()))
                * f64::from(state.camera.zoom.get());
            let y = (f64::from(y) - f64::from(state.camera.origin.y()))
                * f64::from(state.camera.zoom.get());
            [
                x * 2.0 / f64::from(state.viewport.canvas.width()) - 1.0,
                1.0 - y * 2.0 / f64::from(state.viewport.canvas.height()),
            ]
        });
        for ([x, y], [u, v]) in points.into_iter().zip([
            [uv[0], uv[1]],
            [uv[2], uv[1]],
            [uv[2], uv[3]],
            [uv[0], uv[3]],
        ]) {
            let x = x.to_f32().unwrap();
            let y = y.to_f32().unwrap();
            for value in [
                x, y, u, v, colors[0], colors[1], colors[2], colors[3], bounds[0], bounds[1],
                bounds[2], bounds[3],
            ] {
                self.bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
    }
}

fn linear(byte: u8) -> f32 {
    let value = f32::from(byte) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
