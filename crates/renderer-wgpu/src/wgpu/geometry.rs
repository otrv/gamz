use super::{RendererError, Texture, error};
use math::{Rect, Vec2, Vec4};
use num_traits::ToPrimitive;
use platform_api::render::{Frame, RectStyle, RenderCommand, Sampling, TexturedQuad};

pub(super) const VERTEX_BYTES: usize = 48;

#[derive(Clone, Copy)]
pub(super) struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    canvas: Vec2,
}

impl Viewport {
    pub(super) fn new(canvas: Vec2, width: u32, height: u32) -> Option<Self> {
        let width = f64::from(width);
        let height = f64::from(height);
        let scale = (width / f64::from(canvas.x)).min(height / f64::from(canvas.y));
        let fitted_width = (f64::from(canvas.x) * scale).min(width);
        let fitted_height = (f64::from(canvas.y) * scale).min(height);
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

    fn clip(self, rect: Option<Rect>) -> [u32; 4] {
        let width = f64::from(self.canvas.x);
        let height = f64::from(self.canvas.y);
        let [left, top, right, bottom] = rect.map_or([0.0, 0.0, width, height], |rect| {
            let x = f64::from(rect.origin.x);
            let y = f64::from(rect.origin.y);
            [x, y, x + f64::from(rect.size.x), y + f64::from(rect.size.y)]
        });
        let x = (f64::from(self.x) + left.clamp(0.0, width) / width * f64::from(self.width))
            .ceil()
            .to_u32()
            .unwrap();
        let y = (f64::from(self.y) + top.clamp(0.0, height) / height * f64::from(self.height))
            .ceil()
            .to_u32()
            .unwrap();
        let right = (f64::from(self.x) + right.clamp(0.0, width) / width * f64::from(self.width))
            .floor()
            .to_u32()
            .unwrap();
        let bottom = (f64::from(self.y)
            + bottom.clamp(0.0, height) / height * f64::from(self.height))
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
    origin: Vec2,
    zoom: f32,
    viewport: Viewport,
    clip: [u32; 4],
}

pub(super) struct Geometry {
    pub bytes: Vec<u8>,
    pub batches: Vec<Batch>,
    max_quads: usize,
}

impl Geometry {
    pub(super) fn new(max_quads: usize) -> Result<Self, RendererError> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(max_quads * 4 * VERTEX_BYTES)
            .map_err(error)?;
        let mut batches = Vec::new();
        batches.try_reserve_exact(max_quads).map_err(error)?;
        Ok(Self {
            bytes,
            batches,
            max_quads,
        })
    }

    pub(super) fn build(
        &mut self,
        frame: &Frame<'_>,
        viewport: Viewport,
        textures: &[Option<Texture>],
    ) -> Result<(), RendererError> {
        self.bytes.clear();
        self.batches.clear();
        let mut state = State {
            origin: Vec2 { x: 0.0, y: 0.0 },
            zoom: 1.0,
            viewport,
            clip: viewport.clip(None),
        };
        self.solid(
            &mut state,
            [
                0.0,
                0.0,
                f64::from(frame.canvas_size.x),
                f64::from(frame.canvas_size.y),
            ],
            frame.clear_color,
        )?;
        for command in frame.commands {
            match *command {
                RenderCommand::SetCamera { origin, zoom } => {
                    state.origin = origin;
                    state.zoom = zoom;
                }
                RenderCommand::SetClip(rect) => state.clip = viewport.clip(rect),
                RenderCommand::DrawQuads {
                    texture,
                    quads,
                    tint,
                    sampling,
                } => {
                    let slot = usize::from(texture.0);
                    let texture = textures[slot].as_ref().expect("validated resident texture");
                    for &quad in quads {
                        self.textured(&mut state, quad, slot, texture, sampling, tint)?;
                    }
                }
                RenderCommand::DrawRect { rect, style } => {
                    let x = f64::from(rect.origin.x);
                    let y = f64::from(rect.origin.y);
                    let w = f64::from(rect.size.x);
                    let h = f64::from(rect.size.y);
                    match style {
                        RectStyle::Filled(color) => self.solid(&mut state, [x, y, w, h], color)?,
                        RectStyle::Outline { color, width } => {
                            let stroke = f64::from(width).min(w * 0.5).min(h * 0.5);
                            for bounds in [
                                [x, y, w, stroke],
                                [x, y + h - stroke, w, stroke],
                                [x, y + stroke, stroke, h - 2.0 * stroke],
                                [x + w - stroke, y + stroke, stroke, h - 2.0 * stroke],
                            ] {
                                self.solid(&mut state, bounds, color)?;
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
                    let start = [f64::from(start.x), f64::from(start.y)];
                    let end = [f64::from(end.x), f64::from(end.y)];
                    let dx = end[0] - start[0];
                    let dy = end[1] - start[1];
                    let length = dx.hypot(dy);
                    if length > 0.0 {
                        let nx = -dy / length * f64::from(width) * 0.5;
                        let ny = dx / length * f64::from(width) * 0.5;
                        self.quad(
                            &state,
                            [
                                [start[0] + nx, start[1] + ny],
                                [end[0] + nx, end[1] + ny],
                                [end[0] - nx, end[1] - ny],
                                [start[0] - nx, start[1] - ny],
                            ],
                            None,
                            Sampling::Nearest,
                            [[0.0, 0.0, 1.0, 1.0], [0.5; 4]],
                            color,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn solid(
        &mut self,
        state: &mut State,
        [x, y, w, h]: [f64; 4],
        color: Vec4,
    ) -> Result<(), RendererError> {
        if w == 0.0 || h == 0.0 {
            return Ok(());
        }
        self.quad(
            state,
            [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
            None,
            Sampling::Nearest,
            [[0.0, 0.0, 1.0, 1.0], [0.5; 4]],
            color,
        )
    }

    fn textured(
        &mut self,
        state: &mut State,
        quad: TexturedQuad,
        slot: usize,
        texture: &Texture,
        sampling: Sampling,
        tint: Vec4,
    ) -> Result<(), RendererError> {
        let source = quad.source;
        if source.width == 0 || source.height == 0 || quad.size.x == 0.0 || quad.size.y == 0.0 {
            return Ok(());
        }
        let tw = texture.width.to_f32().unwrap();
        let th = texture.height.to_f32().unwrap();
        let uv = [
            f32::from(source.x) / tw,
            f32::from(source.y) / th,
            (f32::from(source.x) + f32::from(source.width)) / tw,
            (f32::from(source.y) + f32::from(source.height)) / th,
        ];
        let bounds = [
            uv[0] + 0.5 / tw,
            uv[1] + 0.5 / th,
            uv[2] - 0.5 / tw,
            uv[3] - 0.5 / th,
        ];
        let (sin, cos) = f64::from(quad.rotation.get()).sin_cos();
        let w = f64::from(quad.size.x) * 0.5;
        let h = f64::from(quad.size.y) * 0.5;
        let points = [[-w, -h], [w, -h], [w, h], [-w, h]].map(|[x, y]| {
            [
                f64::from(quad.center.x) + x * cos - y * sin,
                f64::from(quad.center.y) + x * sin + y * cos,
            ]
        });
        self.quad(state, points, Some(slot), sampling, [uv, bounds], tint)
    }

    fn quad(
        &mut self,
        state: &State,
        points: [[f64; 2]; 4],
        texture: Option<usize>,
        sampling: Sampling,
        [uv, bounds]: [[f32; 4]; 2],
        color: Vec4,
    ) -> Result<(), RendererError> {
        if state.clip[2] == 0 || state.clip[3] == 0 {
            return Ok(());
        }
        let index = self.bytes.len() / (VERTEX_BYTES * 4);
        assert!(index < self.max_quads, "validated quad budget");
        let index = u32::try_from(index).unwrap();
        let compatible = self.batches.last().is_some_and(|batch| {
            batch.texture == texture && batch.sampling == sampling && batch.clip == state.clip
        });
        if compatible {
            self.batches.last_mut().unwrap().end = index + 1;
        } else {
            assert!(self.batches.len() < self.max_quads);
            self.batches.push(Batch {
                texture,
                sampling,
                clip: state.clip,
                start: index,
                end: index + 1,
            });
        }
        let colors = [linear(color.x), linear(color.y), linear(color.z), color.w];
        let points = points.map(|[x, y]| {
            let x = (x - f64::from(state.origin.x)) * f64::from(state.zoom);
            let y = (y - f64::from(state.origin.y)) * f64::from(state.zoom);
            [
                x * 2.0 / f64::from(state.viewport.canvas.x) - 1.0,
                1.0 - y * 2.0 / f64::from(state.viewport.canvas.y),
            ]
        });
        let vertices = points.into_iter().zip([
            [uv[0], uv[1]],
            [uv[2], uv[1]],
            [uv[2], uv[3]],
            [uv[0], uv[3]],
        ]);
        for ([x, y], [u, v]) in vertices {
            let x = x
                .to_f32()
                .filter(|value| value.is_finite())
                .ok_or_else(|| error("transformed vertex exceeds GPU range"))?;
            let y = y
                .to_f32()
                .filter(|value| value.is_finite())
                .ok_or_else(|| error("transformed vertex exceeds GPU range"))?;
            for value in [
                x, y, u, v, colors[0], colors[1], colors[2], colors[3], bounds[0], bounds[1],
                bounds[2], bounds[3],
            ] {
                self.bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
        Ok(())
    }
}

fn linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
