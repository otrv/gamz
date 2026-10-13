mod assets;

use assets::{Assets, ids};
use game_core::entity::{EntityId, EntityStorage};
use game_core::render::RenderCommands;
use game_memory::Arena;
use platform_api::audio::{AudioBuffer, StereoFrame};
use platform_api::input::FrameInput;
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::render::{
    Camera, Canvas, CanvasRect, Color, Frame, GlyphQuad, PixelRect, Point, Radians, Rect,
    RectStyle, RenderCommand, Sampling, Scale, Size, Sprite, StrokeWidth, TextureUpload, Transform,
};
use platform_api::services::{PlatformApi, StartupError};

const ENTITY_COUNT: usize = 512;
const COMMAND_COUNT: usize = ENTITY_COUNT + 128;
const GLYPH_COUNT: usize = 256;

#[derive(Clone, Copy, Debug)]
struct Particle {
    x: f32,
    y: f32,
    speed: f32,
    rotation: f32,
    tint: Color,
}

pub struct GameState {
    entities: EntityStorage<Particle, ENTITY_COUNT>,
    handles: [EntityId<Particle>; ENTITY_COUNT],
    assets: Assets,
    phase: f32,
    recycle_time: f32,
    recycle_cursor: usize,
}

impl platform_api::Game for GameState {
    fn initialize<'p, 't>(
        persistent: PersistentMemory<'p>,
        _transient: TransientMemory<'t>,
        _api: &PlatformApi,
    ) -> Result<(&'p mut Self, impl AsRef<[TextureUpload<'t>]>), StartupError> {
        let PersistentMemory { bytes: persistent } = persistent;
        let required = core::mem::size_of::<Self>() + core::mem::align_of::<Self>() - 1;
        if persistent.len() < required {
            return Err(StartupError::InsufficientPersistentMemory {
                required,
                available: persistent.len(),
            });
        }
        let (assets, uploads) = Assets::initialize().map_err(|_| StartupError::InvalidAsset {
            path: "embedded assets.pack",
        })?;
        let mut entities = EntityStorage::new();
        let handles = core::array::from_fn(|index| {
            let column = u16::try_from(index % 32).unwrap();
            let row = u16::try_from(index / 32).unwrap();
            entities
                .insert(Particle {
                    x: 24.0 + f32::from(column) * 38.0,
                    y: 110.0 + f32::from(row) * 34.0,
                    speed: 18.0 + f32::from(row) * 2.0,
                    rotation: f32::from(column) * 0.1,
                    tint: match index % 3 {
                        0 => Color(112, 230, 204, 220),
                        1 => Color(255, 190, 110, 230),
                        _ => Color(156, 170, 255, 210),
                    },
                })
                .expect("entity budget matches initialization")
        });
        let state = Arena::new(persistent).push(Self {
            entities,
            handles,
            assets,
            phase: 0.0,
            recycle_time: 0.0,
            recycle_cursor: 0,
        });
        Ok((state, uploads))
    }

    fn update<'a>(&mut self, memory: TransientMemory<'a>, input: &FrameInput) -> Frame<'a> {
        advance(self, input.dt.seconds().min(0.1));
        let TransientMemory { bytes: memory } = memory;
        let mut arena = Arena::new(memory);
        let positions = arena.push_slice(ENTITY_COUNT, Point::new(0.0, 0.0));
        for (position, (_, particle)) in positions.iter_mut().zip(self.entities.iter()) {
            *position = Point::new(particle.x, particle.y);
        }
        let storage = arena.push_slice(COMMAND_COUNT, RenderCommand::SetClip(None));
        let glyphs = arena.push_slice(
            GLYPH_COUNT,
            GlyphQuad {
                source: PixelRect::new(0, 0, 0, 0),
                transform: transform(Point::new(0.0, 0.0), 0.0, 0.0),
            },
        );
        let mut commands = RenderCommands::new(storage, glyphs);
        commands.push(RenderCommand::SetClip(Some(CanvasRect::new(Rect::new(
            Point::new(24.0, 96.0),
            Size::new(1232.0, 568.0),
        )))));
        let sweep = 1.0 - (self.phase - 1.0).abs();
        commands.push(RenderCommand::SetCamera(Camera {
            origin: Point::new(sweep * 36.0, sweep * 12.0),
            zoom: Scale::new(1.0 + sweep * 0.025),
        }));
        for column in 0..33_u16 {
            let x = f32::from(column) * 40.0;
            commands.push(RenderCommand::DrawLine {
                start: Point::new(x, 90.0),
                end: Point::new(x, 680.0),
                width: StrokeWidth::new(1.0),
                color: Color(33, 48, 65, 255),
            });
        }
        for row in 0..16_u16 {
            commands.push(RenderCommand::DrawRect {
                rect: Rect::new(
                    Point::new(0.0, 100.0 + f32::from(row) * 36.0),
                    Size::new(1280.0, 16.0),
                ),
                style: RectStyle::Filled(Color(24, 35, 49, 255)),
            });
        }
        for pair in positions.as_chunks::<16>().0 {
            commands.push(RenderCommand::DrawLine {
                start: pair[0],
                end: pair[15],
                width: StrokeWidth::new(1.5),
                color: Color(78, 108, 130, 110),
            });
        }
        let texture = self.assets.image(ids::SPRITE);
        for (_, particle) in self.entities.iter() {
            commands.push(RenderCommand::DrawSprite(Sprite {
                texture,
                source: PixelRect::new(0, 0, 16, 16),
                transform: transform(Point::new(particle.x, particle.y), 20.0, particle.rotation),
                tint: particle.tint,
                sampling: if particle.speed > 32.0 {
                    Sampling::Linear
                } else {
                    Sampling::Nearest
                },
            }));
        }
        commands.push(RenderCommand::SetCamera(Camera::default()));
        commands.push(RenderCommand::SetClip(None));
        commands.draw_text(
            self.assets.font(ids::FONT),
            "GAMZ / PARTICLE GARDEN",
            Point::new(28.0, 24.0),
            Scale::new(3.0),
            Color(227, 238, 245, 255),
            Sampling::Nearest,
        );
        commands.draw_text(
            self.assets.font(ids::FONT),
            "512 SPRITES / HANDLES / CAMERA / CLIP / TEXT",
            Point::new(28.0, 690.0),
            Scale::new(1.5),
            Color(133, 168, 186, 255),
            Sampling::Nearest,
        );
        Frame {
            canvas: Canvas::new(1280, 720),
            clear: Color(16, 23, 34, 255),
            commands: commands.finish(),
        }
    }

    fn update_audio(&mut self, mut buffer: AudioBuffer<'_>) {
        buffer.frames_mut().fill(StereoFrame::default());
    }
}

fn advance(state: &mut GameState, dt: f32) {
    state.phase = (state.phase + dt * 0.15) % 2.0;
    state.recycle_time += dt;
    if state.recycle_time >= 0.5 {
        state.recycle_time -= 0.5;
        for _ in 0..16 {
            let handle = &mut state.handles[state.recycle_cursor];
            let mut particle = state
                .entities
                .remove(*handle)
                .expect("live particle handle");
            particle.x = (particle.x + 83.0) % 1240.0;
            *handle = state
                .entities
                .insert(particle)
                .expect("removed slot is available");
            state.recycle_cursor = (state.recycle_cursor + 1) % ENTITY_COUNT;
        }
    }
    for (_, particle) in state.entities.iter_mut() {
        particle.x = (particle.x + particle.speed * dt) % 1240.0;
        particle.rotation = (particle.rotation + dt * 0.6) % core::f32::consts::TAU;
    }
}

fn transform(center: Point, side: f32, rotation: f32) -> Transform {
    Transform {
        center,
        size: Size::new(side, side),
        rotation: Radians::new(rotation),
    }
}
