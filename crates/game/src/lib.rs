#![no_std]

mod hud;

use core::mem::{align_of, size_of};

use game_core::font::Font;
use game_core::render::{self, RenderError};
use math::{Mat4, Radians, Vec2, Vec3, Vec4};
use memory::Arena;
use platform_api::audio::StereoSample;
use platform_api::input::{ButtonPosition, Input, PlatformEvent};
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::render::{
    Frame, RenderCommand, RenderCommands, TextureId, TextureUpload, TexturedQuad, Vertex3D,
};
use platform_api::{FrameError, Game, Platform, StartupError};

const NEAR_LABEL: &str = "NEAR TRIANGLE   PRESS START";
const AWAY_LABEL: &str = "AWAY TRIANGLE   PRESS START";
const TRIANGLE: [Vertex3D; 3] = [
    Vertex3D {
        position: Vec3::new(-1.5, -1.0, 0.0),
        color: Vec4::new(1.0, 0.25, 0.2, 1.0),
    },
    Vertex3D {
        position: Vec3::new(0.0, 1.0, 0.0),
        color: Vec4::new(0.2, 1.0, 0.5, 1.0),
    },
    Vertex3D {
        position: Vec3::new(1.5, -1.0, 0.0),
        color: Vec4::new(0.3, 0.4, 1.0, 1.0),
    },
];

enum Distance {
    Near,
    Away,
}

pub struct GameState {
    font: Font,
    transient_capacity: usize,
    distance: Distance,
}

impl Game for GameState {
    fn initialize<'persistent, Context>(
        memory: PersistentMemory<'persistent>,
        transient_capacity: usize,
        platform: &mut Platform<'_, Context>,
    ) -> Result<&'persistent mut Self, StartupError> {
        let required = size_of::<Self>() + align_of::<Self>() - 1;
        if memory.bytes.len() < required {
            return Err(StartupError::InsufficientPersistentMemory {
                required,
                available: memory.bytes.len(),
            });
        }
        let required_transient = size_of::<TexturedQuad>() * NEAR_LABEL.len().max(AWAY_LABEL.len())
            + align_of::<TexturedQuad>()
            - 1;
        if transient_capacity < required_transient {
            return Err(StartupError::InsufficientTransientMemory {
                required: required_transient,
                available: transient_capacity,
            });
        }
        let state = Self {
            font: hud::font().map_err(|_| StartupError::Game { code: 1 })?,
            transient_capacity,
            distance: Distance::Near,
        };
        let image = hud::image();
        (platform.upload_texture)(
            platform.context,
            TextureUpload {
                id: TextureId(0),
                width: u32::from(image.width),
                height: u32::from(image.height),
                rgba: image.rgba,
            },
        )
        .map_err(StartupError::Texture)?;
        Ok(Arena::new(memory.bytes).push(state))
    }

    fn update<'frame, Context>(
        &mut self,
        input: &Input<'_>,
        memory: TransientMemory<'frame>,
        commands: RenderCommands<'frame>,
        platform: &mut Platform<'_, Context>,
    ) -> Result<Frame<'frame>, FrameError> {
        if memory.bytes.len() < self.transient_capacity {
            return Err(FrameError::InsufficientTransientMemory {
                required: self.transient_capacity,
                available: memory.bytes.len(),
            });
        }
        if input.events.contains(&PlatformEvent::CloseRequested) {
            (platform.exit)(platform.context);
        }
        let button = input.actions.start;
        let presses = button.half_transitions / 2
            + u32::from(
                button.ended == ButtonPosition::Down && !button.half_transitions.is_multiple_of(2),
            );
        if !presses.is_multiple_of(2) {
            self.distance = match self.distance {
                Distance::Near => Distance::Away,
                Distance::Away => Distance::Near,
            };
        }
        draw(self, memory, commands).map_err(|RenderError::CommandCapacityExceeded| {
            FrameError::RenderCommandCapacityExceeded
        })
    }

    fn update_audio(&mut self, _sample_rate_hz: u32, samples: &mut [StereoSample]) {
        samples.fill(StereoSample { left: 0, right: 0 });
    }
}

fn draw<'frame>(
    state: &GameState,
    memory: TransientMemory<'frame>,
    mut commands: RenderCommands<'frame>,
) -> Result<Frame<'frame>, RenderError> {
    let TransientMemory { bytes } = memory;
    let mut arena = Arena::new(bytes);
    let (distance, label) = match state.distance {
        Distance::Near => (3.0, NEAR_LABEL),
        Distance::Away => (6.0, AWAY_LABEL),
    };
    render::push(
        &mut commands,
        RenderCommand::SetCamera3D {
            view_projection: Mat4::perspective_lh(
                Radians::new(core::f32::consts::FRAC_PI_2),
                1280.0 / 720.0,
                0.1,
                100.0,
            ),
        },
    )?;
    render::push(
        &mut commands,
        RenderCommand::DrawMesh {
            vertices: &TRIANGLE,
            indices: &[0, 1, 2],
            model: Mat4::from_translation(Vec3::new(0.0, 0.0, distance)),
        },
    )?;
    hud::draw(
        &state.font,
        label,
        Vec4::new(227.0 / 255.0, 238.0 / 255.0, 245.0 / 255.0, 1.0),
        &mut arena,
        &mut commands,
    )?;
    Ok(Frame {
        canvas_size: Vec2::new(1280.0, 720.0),
        clear_color: Vec4::new(16.0 / 255.0, 23.0 / 255.0, 34.0 / 255.0, 1.0),
        commands: &commands.storage[..commands.len],
    })
}
