#![no_std]

mod hud;

use core::mem::{align_of, size_of};

use game_core::entity::{EntityId, EntityStorage};
use game_core::font::Font;
use game_core::render::RenderCommands;
use game_memory::Arena;
use math::{Radians, Rect, Vec2, Vec2x4, Vec4};
use platform_api::audio::StereoSample;
use platform_api::input::{ButtonPosition, ButtonState, Input, PlatformEvent};
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::render::{
    Frame, RectStyle, RenderCommand, TextureId, TextureUpload, TexturedQuad,
};
use platform_api::{FrameError, Game, Platform, StartupError};

const BRICK_COUNT: usize = 32;
const MAX_RENDER_COMMANDS: usize = BRICK_COUNT + 12;
const READY_LABEL: &str = "PRESS START";
const PLAYING_LABEL: &str = "PLAYING";
const WON_LABEL: &str = "YOU WON";
const LOST_LABEL: &str = "GAME OVER";
const BRICK_HALF: Vec2 = Vec2::new(68.0, 14.0);
const BALL_HALF: f32 = 6.0;
const BALL_SPEED: f32 = 420.0;
const PADDLE_HALF: f32 = 72.0;
const PADDLE_Y: f32 = 650.0;
const LEFT: f32 = 24.0;
const RIGHT: f32 = 1256.0;
const TOP: f32 = 72.0;
const MAX_IMPACTS: usize = 8;

#[derive(Clone, Copy, Debug)]
struct Brick {
    center: Vec2,
    color: Vec4,
}

#[derive(Clone, Copy)]
enum Phase {
    Ready,
    Playing { ball: Vec2, velocity: Vec2 },
    Won,
    Lost,
}

pub struct GameState {
    bricks: EntityStorage<Brick, BRICK_COUNT>,
    font: Font,
    transient_capacity: usize,
    paddle: Vec2,
    phase: Phase,
    lives: u8,
}

impl GameState {
    fn reset(&mut self) {
        self.bricks.clear();
        for row in 0..4_u16 {
            for column in 0..8_u16 {
                let _ = self
                    .bricks
                    .insert(Brick {
                        center: Vec2::new(
                            115.0 + f32::from(column) * 150.0,
                            132.0 + f32::from(row) * 40.0,
                        ),
                        color: [
                            Vec4::new(248.0 / 255.0, 112.0 / 255.0, 108.0 / 255.0, 1.0),
                            Vec4::new(1.0, 190.0 / 255.0, 110.0 / 255.0, 1.0),
                            Vec4::new(112.0 / 255.0, 230.0 / 255.0, 204.0 / 255.0, 1.0),
                            Vec4::new(156.0 / 255.0, 170.0 / 255.0, 1.0, 1.0),
                        ][usize::from(row)],
                    })
                    .expect("brick budget matches the grid");
            }
        }
        self.paddle = Vec2::new(640.0, PADDLE_Y);
        self.phase = Phase::Ready;
        self.lives = 3;
    }

    fn ready_ball(&self) -> Vec2 {
        Vec2::new(self.paddle.x, PADDLE_Y - BALL_HALF)
    }
}

impl Game for GameState {
    fn initialize<'p, Context>(
        memory: PersistentMemory<'p>,
        transient_capacity: usize,
        platform: &mut Platform<'_, Context>,
    ) -> Result<&'p mut Self, StartupError> {
        let required = size_of::<Self>() + align_of::<Self>() - 1;
        if memory.bytes.len() < required {
            return Err(StartupError::InsufficientPersistentMemory {
                required,
                available: memory.bytes.len(),
            });
        }
        let max_glyphs = [READY_LABEL, PLAYING_LABEL, WON_LABEL, LOST_LABEL]
            .into_iter()
            .map(str::len)
            .max()
            .expect("the HUD has labels");
        let required_transient = size_of::<RenderCommand<'_>>() * MAX_RENDER_COMMANDS
            + (align_of::<RenderCommand<'_>>() - 1)
            + size_of::<TexturedQuad>() * max_glyphs
            + (align_of::<TexturedQuad>() - 1);
        if transient_capacity < required_transient {
            return Err(StartupError::InsufficientTransientMemory {
                required: required_transient,
                available: transient_capacity,
            });
        }
        let mut state = Self {
            bricks: EntityStorage::new(),
            font: hud::font().map_err(|_| StartupError::Game { code: 1 })?,
            transient_capacity,
            paddle: Vec2::new(640.0, PADDLE_Y),
            phase: Phase::Ready,
            lives: 3,
        };
        state.reset();
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

    fn update<'a, Context>(
        &mut self,
        input: &Input<'_>,
        memory: TransientMemory<'a>,
        platform: &mut Platform<'_, Context>,
    ) -> Result<Frame<'a>, FrameError> {
        if memory.bytes.len() < self.transient_capacity {
            return Err(FrameError::InsufficientTransientMemory {
                required: self.transient_capacity,
                available: memory.bytes.len(),
            });
        }
        if input.events.contains(&PlatformEvent::CloseRequested) {
            (platform.exit)(platform.context);
        }
        let dt = input.elapsed.as_secs_f32().min(0.05);
        let start = pressed(input.actions.start);
        if matches!(self.phase, Phase::Won | Phase::Lost) {
            if start {
                self.reset();
            }
        } else {
            let right = u8::from(input.actions.move_right.ended == ButtonPosition::Down);
            let left = u8::from(input.actions.move_left.ended == ButtonPosition::Down);
            let movement = if right != 0 || left != 0 {
                f32::from(right) - f32::from(left)
            } else if input.actions.movement.x.is_finite() {
                input.actions.movement.x.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            let x = self.paddle.x + movement * 700.0 * dt;
            self.paddle = Vec2::new(x.clamp(LEFT + PADDLE_HALF, RIGHT - PADDLE_HALF), PADDLE_Y);
            if matches!(self.phase, Phase::Ready) && start {
                self.phase = Phase::Playing {
                    ball: self.ready_ball(),
                    velocity: Vec2::new(0.0, -BALL_SPEED).rotate(Radians::new(0.35)),
                };
            }
            if dt > 0.0 {
                advance(self, dt);
            }
        }
        Ok(draw(self, memory))
    }

    fn update_audio(&mut self, _sample_rate_hz: u32, samples: &mut [StereoSample]) {
        samples.fill(StereoSample { left: 0, right: 0 });
    }
}

fn pressed(button: ButtonState) -> bool {
    button.half_transitions > 1
        || (button.ended == ButtonPosition::Down && button.half_transitions != 0)
}

#[derive(Clone, Copy)]
enum Target {
    Wall,
    Paddle,
    Brick(EntityId<Brick>),
}

struct Hit {
    fraction: f32,
    normal: Vec2,
    target: Target,
}

fn consider(hit: &mut Option<Hit>, fraction: f32, normal: Vec2, target: Target) {
    if (0.0..=1.0).contains(&fraction) && hit.as_ref().is_none_or(|hit| fraction < hit.fraction) {
        *hit = Some(Hit {
            fraction,
            normal,
            target,
        });
    }
}

fn advance(state: &mut GameState, mut remaining: f32) {
    let Phase::Playing {
        mut ball,
        mut velocity,
    } = state.phase
    else {
        return;
    };
    for _ in 0..MAX_IMPACTS {
        let delta = velocity * remaining;
        let Some(hit) = find_hit(state, ball, delta) else {
            ball = ball + delta;
            if ball.y > 720.0 + BALL_HALF {
                state.lives -= 1;
                state.phase = if state.lives == 0 {
                    Phase::Lost
                } else {
                    Phase::Ready
                };
            } else {
                state.phase = Phase::Playing { ball, velocity };
            }
            return;
        };
        ball = ball.lerp(ball + delta, hit.fraction);
        match hit.target {
            Target::Paddle => {
                let offset = ((ball.x - state.paddle.x) / PADDLE_HALF).clamp(-1.0, 1.0);
                velocity = Vec2::new(0.0, -1.0).rotate(Radians::new(offset * 1.05)) * BALL_SPEED;
            }
            Target::Wall | Target::Brick(_) => {
                let reflected = velocity - hit.normal * (2.0 * velocity.dot(hit.normal));
                velocity = reflected
                    .try_normalize()
                    .expect("reflection preserves nonzero finite velocity")
                    * BALL_SPEED;
                if let Target::Brick(id) = hit.target {
                    let _ = state
                        .bricks
                        .remove(id)
                        .expect("collision selected a live brick");
                    if state.bricks.is_empty() {
                        state.phase = Phase::Won;
                        return;
                    }
                }
            }
        }
        remaining *= 1.0 - hit.fraction;
        if remaining <= 0.0 {
            state.phase = Phase::Playing { ball, velocity };
            return;
        }
    }
    panic!("frame collision budget exhausted");
}

fn find_hit(state: &GameState, ball: Vec2, delta: Vec2) -> Option<Hit> {
    let mut hit = None;
    if delta.x < 0.0 {
        consider(
            &mut hit,
            (LEFT + BALL_HALF - ball.x) / delta.x,
            Vec2::new(1.0, 0.0),
            Target::Wall,
        );
    } else if delta.x > 0.0 {
        consider(
            &mut hit,
            (RIGHT - BALL_HALF - ball.x) / delta.x,
            Vec2::new(-1.0, 0.0),
            Target::Wall,
        );
    }
    if delta.y < 0.0 {
        consider(
            &mut hit,
            (TOP + BALL_HALF - ball.y) / delta.y,
            Vec2::new(0.0, 1.0),
            Target::Wall,
        );
    } else if delta.y > 0.0 {
        let fraction = (PADDLE_Y - BALL_HALF - ball.y) / delta.y;
        let x = ball.x + delta.x * fraction;
        if (x - state.paddle.x).abs() <= PADDLE_HALF + BALL_HALF {
            consider(&mut hit, fraction, Vec2::new(0.0, -1.0), Target::Paddle);
        }
    }
    let half = BRICK_HALF + Vec2::new(BALL_HALF, BALL_HALF);
    let reach = half.length() + delta.length();
    let mut bricks = state.bricks.iter();
    for _ in 0..BRICK_COUNT.div_ceil(4) {
        let batch: [_; 4] = core::array::from_fn(|_| bricks.next());
        if batch[0].is_none() {
            break;
        }
        let centers = Vec2x4::from_array(
            batch.map(|brick| brick.map_or(Vec2::ZERO, |(_, brick)| brick.center)),
        );
        let offsets = Vec2x4::from_array([ball; 4]) - centers;
        let distances = offsets.dot(offsets);
        let brick_distances = batch.into_iter().zip(distances);
        for (brick, distance) in brick_distances {
            if let Some((id, brick)) = brick
                && distance <= reach * reach
                && let Some((fraction, normal)) = sweep(ball, delta, brick.center, half)
            {
                consider(&mut hit, fraction, normal, Target::Brick(id));
            }
        }
    }
    hit
}

fn sweep(ball: Vec2, delta: Vec2, center: Vec2, half: Vec2) -> Option<(f32, Vec2)> {
    let mut entry = 0.0_f32;
    let mut exit = 1.0_f32;
    let mut normal = Vec2::ZERO;
    for (position, movement, low, high, axis) in [
        (
            ball.x,
            delta.x,
            center.x - half.x,
            center.x + half.x,
            Vec2::new(1.0, 0.0),
        ),
        (
            ball.y,
            delta.y,
            center.y - half.y,
            center.y + half.y,
            Vec2::new(0.0, 1.0),
        ),
    ] {
        if movement == 0.0 {
            if position < low || position > high {
                return None;
            }
            continue;
        }
        let first = (low - position) / movement;
        let second = (high - position) / movement;
        let near = first.min(second);
        if near >= entry {
            entry = near;
            normal = axis * if movement > 0.0 { -1.0 } else { 1.0 };
        }
        exit = exit.min(first.max(second));
        if entry > exit {
            return None;
        }
    }
    (normal.dot(delta) < 0.0).then_some((entry, normal))
}

fn draw<'a>(state: &GameState, memory: TransientMemory<'a>) -> Frame<'a> {
    let TransientMemory { bytes } = memory;
    let mut arena = Arena::new(bytes);
    let storage = arena.push_slice(MAX_RENDER_COMMANDS, RenderCommand::SetClip(None));
    let mut commands = RenderCommands::new(storage);
    draw_court(state, &mut commands);
    let (ball, status) = match state.phase {
        Phase::Ready => (
            Some(state.ready_ball()),
            Vec4::new(1.0, 190.0 / 255.0, 110.0 / 255.0, 1.0),
        ),
        Phase::Playing { ball, .. } => (
            Some(ball),
            Vec4::new(112.0 / 255.0, 230.0 / 255.0, 204.0 / 255.0, 1.0),
        ),
        Phase::Won => (
            None,
            Vec4::new(112.0 / 255.0, 230.0 / 255.0, 204.0 / 255.0, 1.0),
        ),
        Phase::Lost => (
            None,
            Vec4::new(248.0 / 255.0, 112.0 / 255.0, 108.0 / 255.0, 1.0),
        ),
    };
    let label = match state.phase {
        Phase::Ready => READY_LABEL,
        Phase::Playing { .. } => PLAYING_LABEL,
        Phase::Won => WON_LABEL,
        Phase::Lost => LOST_LABEL,
    };
    hud::draw(&state.font, label, status, &mut arena, &mut commands)
        .expect("frame command budget includes the HUD");
    if let Some(ball) = ball {
        rect(
            &mut commands,
            ball - Vec2::new(BALL_HALF, BALL_HALF),
            Vec2::new(BALL_HALF * 2.0, BALL_HALF * 2.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
        );
        if matches!(state.phase, Phase::Ready) {
            let aim = Vec2::new(0.0, -48.0).rotate(Radians::new(0.35));
            commands
                .push(RenderCommand::DrawLine {
                    start: ball,
                    end: ball + aim,
                    width: 2.0,
                    color: status,
                })
                .expect("frame command budget includes the aim line");
        }
    } else {
        rect(
            &mut commands,
            Vec2::new(480.0, 352.0),
            Vec2::new(320.0, 16.0),
            status,
        );
    }
    Frame {
        canvas_size: Vec2::new(1280.0, 720.0),
        clear_color: Vec4::new(16.0 / 255.0, 23.0 / 255.0, 34.0 / 255.0, 1.0),
        commands: commands.finish(),
    }
}

fn draw_court(state: &GameState, commands: &mut RenderCommands<'_>) {
    let wall = Vec4::new(73.0 / 255.0, 87.0 / 255.0, 108.0 / 255.0, 1.0);
    for (x, y, width, height) in [
        (16.0, 64.0, 1248.0, 8.0),
        (16.0, TOP, 8.0, 632.0),
        (RIGHT, TOP, 8.0, 632.0),
    ] {
        rect(commands, Vec2::new(x, y), Vec2::new(width, height), wall);
    }
    for (_, brick) in state.bricks.iter() {
        rect(
            commands,
            brick.center - BRICK_HALF,
            BRICK_HALF * 2.0,
            brick.color,
        );
    }
    rect(
        commands,
        Vec2::new(state.paddle.x - PADDLE_HALF, PADDLE_Y),
        Vec2::new(PADDLE_HALF * 2.0, 14.0),
        Vec4::new(227.0 / 255.0, 238.0 / 255.0, 245.0 / 255.0, 1.0),
    );
    for life in 0..state.lives {
        rect(
            commands,
            Vec2::new(28.0 + f32::from(life) * 24.0, 28.0),
            Vec2::new(14.0, 14.0),
            Vec4::new(112.0 / 255.0, 230.0 / 255.0, 204.0 / 255.0, 1.0),
        );
    }
}

fn rect(commands: &mut RenderCommands<'_>, origin: Vec2, size: Vec2, color: Vec4) {
    commands
        .push(RenderCommand::DrawRect {
            rect: Rect { origin, size },
            style: RectStyle::Filled(color),
        })
        .expect("frame command budget includes court rectangles");
}
