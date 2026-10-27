#![no_std]

mod hud;
mod scene;

use core::mem::{align_of, size_of};
use core::time::Duration;

use game_core::font::Font;
use game_core::render::RenderError;
use math::{Vec2, Vec3, Vec4};
use memory::Arena;
use platform_api::audio::StereoSample;
use platform_api::input::{ButtonPosition, Input, PlatformEvent};
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::render::{Frame, RenderCommands, TextureId, TextureUpload, TexturedQuad};
use platform_api::{FrameError, Game, Platform, StartupError};
use scene::DrawOrder;

const NORMAL_LABEL: &str = "WASD MOVE   NORMAL   START REVERSES";
const REVERSE_LABEL: &str = "WASD MOVE   REVERSE   START RESTORES";
const PLAYER_SPEED: f32 = 4.0;
const MAX_MOVEMENT_ELAPSED: Duration = Duration::from_millis(100);

pub struct GameState {
    font: Font,
    transient_capacity: usize,
    draw_order: DrawOrder,
    player_position: Vec3,
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
        let required_transient = size_of::<TexturedQuad>()
            * NORMAL_LABEL.len().max(REVERSE_LABEL.len())
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
            draw_order: DrawOrder::Forward,
            player_position: Vec3::new(0.0, 0.0, -2.0),
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
        let elapsed = input.elapsed.min(MAX_MOVEMENT_ELAPSED).as_secs_f32();
        let movement = input.actions.movement;
        let displacement = movement * (PLAYER_SPEED * elapsed / movement.length().max(1.0));
        self.player_position.x += displacement.x;
        self.player_position.z -= displacement.y;
        let button = input.actions.start;
        let presses = button.half_transitions / 2
            + u32::from(
                button.ended == ButtonPosition::Down && !button.half_transitions.is_multiple_of(2),
            );
        if !presses.is_multiple_of(2) {
            self.draw_order = match self.draw_order {
                DrawOrder::Forward => DrawOrder::Reverse,
                DrawOrder::Reverse => DrawOrder::Forward,
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
    let label = match state.draw_order {
        DrawOrder::Forward => NORMAL_LABEL,
        DrawOrder::Reverse => REVERSE_LABEL,
    };
    scene::draw(&state.draw_order, state.player_position, &mut commands)?;
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
