#![no_std]

mod hud;

use game_core::entity::{EntityId, EntityStorage};
use game_core::font::Font;
use game_core::math::{Vec2, Vec2x4};
use game_core::render::RenderCommands;
use game_memory::Arena;
use platform_api::audio::{AudioBuffer, StereoFrame};
use platform_api::input::{ButtonPosition, ButtonState, FrameInput};
use platform_api::math::Radians;
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::render::{
    Canvas, Color, Frame, FrameOutput, Point, Rect, RectStyle, RenderCommand, Size, StrokeWidth,
    TextureId, TextureUpload,
};
use platform_api::services::{PlatformEvent, StartupError};
use platform_api::{Game, Platform};

const BRICK_COUNT: usize = 32;
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
    center: Point,
    color: Color,
}

#[derive(Clone, Copy)]
enum Phase {
    Ready,
    Playing { ball: Point, velocity: Vec2 },
    Won,
    Lost,
}

pub struct GameState {
    bricks: EntityStorage<Brick, BRICK_COUNT>,
    font: Font,
    font_uploaded: bool,
    paddle: Point,
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
                        center: Point::new(
                            115.0 + f32::from(column) * 150.0,
                            132.0 + f32::from(row) * 40.0,
                        ),
                        color: [
                            Color {
                                red: 248,
                                green: 112,
                                blue: 108,
                                alpha: 255,
                            },
                            Color {
                                red: 255,
                                green: 190,
                                blue: 110,
                                alpha: 255,
                            },
                            Color {
                                red: 112,
                                green: 230,
                                blue: 204,
                                alpha: 255,
                            },
                            Color {
                                red: 156,
                                green: 170,
                                blue: 255,
                                alpha: 255,
                            },
                        ][usize::from(row)],
                    })
                    .expect("brick budget matches the grid");
            }
        }
        self.paddle = Point::new(640.0, PADDLE_Y);
        self.phase = Phase::Ready;
        self.lives = 3;
    }

    fn ready_ball(&self) -> Point {
        Point::new(self.paddle.x(), PADDLE_Y - BALL_HALF)
    }
}

impl Game for GameState {
    fn initialize<'p>(
        memory: PersistentMemory<'p>,
        platform: &mut impl Platform<'p>,
    ) -> Result<&'p mut Self, StartupError> {
        let required = core::mem::size_of::<Self>() + core::mem::align_of::<Self>() - 1;
        if memory.bytes.len() < required {
            return Err(StartupError::InsufficientPersistentMemory {
                required,
                available: memory.bytes.len(),
            });
        }
        let (root, io) = memory.bytes.split_at_mut(required);
        let capacity = io.len();
        platform.set_persistent_memory(Arena::new(io).push_slice(capacity, 0_u8));
        let mut state = Self {
            bricks: EntityStorage::new(),
            font: hud::font().map_err(|_| StartupError::Game { code: 1 })?,
            font_uploaded: false,
            paddle: Point::new(640.0, PADDLE_Y),
            phase: Phase::Ready,
            lives: 3,
        };
        state.reset();
        Ok(Arena::new(root).push(state))
    }

    fn update<'p, 'a>(
        &mut self,
        platform: &mut impl Platform<'p>,
        memory: TransientMemory<'a>,
        input: &FrameInput<'_>,
    ) -> FrameOutput<'a> {
        if input.events().contains(&PlatformEvent::CloseRequested) {
            platform.request_exit();
        }
        let dt = input.dt.seconds().min(0.05);
        let start = pressed(input.controller.start);
        if matches!(self.phase, Phase::Won | Phase::Lost) {
            if start {
                self.reset();
            }
        } else {
            let right = u8::from(input.controller.move_right.ended() == ButtonPosition::Down);
            let left = u8::from(input.controller.move_left.ended() == ButtonPosition::Down);
            let x = self.paddle.x() + (f32::from(right) - f32::from(left)) * 700.0 * dt;
            self.paddle = Point::new(x.clamp(LEFT + PADDLE_HALF, RIGHT - PADDLE_HALF), PADDLE_Y);
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
        let output = draw(self, memory);
        self.font_uploaded = true;
        output
    }

    fn update_audio<'p>(&mut self, _platform: &mut impl Platform<'p>, mut buffer: AudioBuffer<'_>) {
        buffer.frames_mut().fill(StereoFrame::default());
    }
}

fn pressed(button: ButtonState) -> bool {
    button.half_transitions() > 1
        || (button.ended() == ButtonPosition::Down && button.half_transitions() != 0)
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
        let position = Vec2::new(ball.x(), ball.y());
        let delta = velocity * remaining;
        let Some(hit) = find_hit(state, ball, delta) else {
            ball = point(position + delta);
            if ball.y() > 720.0 + BALL_HALF {
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
        ball = point(position.lerp(position + delta, hit.fraction));
        match hit.target {
            Target::Paddle => {
                let offset = ((ball.x() - state.paddle.x()) / PADDLE_HALF).clamp(-1.0, 1.0);
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

fn find_hit(state: &GameState, ball: Point, delta: Vec2) -> Option<Hit> {
    let mut hit = None;
    if delta.x < 0.0 {
        consider(
            &mut hit,
            (LEFT + BALL_HALF - ball.x()) / delta.x,
            Vec2::new(1.0, 0.0),
            Target::Wall,
        );
    } else if delta.x > 0.0 {
        consider(
            &mut hit,
            (RIGHT - BALL_HALF - ball.x()) / delta.x,
            Vec2::new(-1.0, 0.0),
            Target::Wall,
        );
    }
    if delta.y < 0.0 {
        consider(
            &mut hit,
            (TOP + BALL_HALF - ball.y()) / delta.y,
            Vec2::new(0.0, 1.0),
            Target::Wall,
        );
    } else if delta.y > 0.0 {
        let fraction = (PADDLE_Y - BALL_HALF - ball.y()) / delta.y;
        let x = ball.x() + delta.x * fraction;
        if (x - state.paddle.x()).abs() <= PADDLE_HALF + BALL_HALF {
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
        let centers = Vec2x4::from_array(batch.map(|brick| {
            brick.map_or(Vec2::ZERO, |(_, brick)| {
                Vec2::new(brick.center.x(), brick.center.y())
            })
        }));
        let offsets = Vec2x4::from_array([Vec2::new(ball.x(), ball.y()); 4]) - centers;
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

fn sweep(ball: Point, delta: Vec2, center: Point, half: Vec2) -> Option<(f32, Vec2)> {
    let mut entry = 0.0_f32;
    let mut exit = 1.0_f32;
    let mut normal = Vec2::ZERO;
    for (position, movement, low, high, axis) in [
        (
            ball.x(),
            delta.x,
            center.x() - half.x,
            center.x() + half.x,
            Vec2::new(1.0, 0.0),
        ),
        (
            ball.y(),
            delta.y,
            center.y() - half.y,
            center.y() + half.y,
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

fn draw<'a>(state: &GameState, memory: TransientMemory<'a>) -> FrameOutput<'a> {
    let TransientMemory { bytes } = memory;
    let mut arena = Arena::new(bytes);
    let storage = arena.push_slice(BRICK_COUNT + 12, RenderCommand::SetClip(None));
    let mut commands = RenderCommands::new(storage);
    draw_court(state, &mut commands);
    let (ball, status) = match state.phase {
        Phase::Ready => (
            Some(state.ready_ball()),
            Color {
                red: 255,
                green: 190,
                blue: 110,
                alpha: 255,
            },
        ),
        Phase::Playing { ball, .. } => (
            Some(ball),
            Color {
                red: 112,
                green: 230,
                blue: 204,
                alpha: 255,
            },
        ),
        Phase::Won => (
            None,
            Color {
                red: 112,
                green: 230,
                blue: 204,
                alpha: 255,
            },
        ),
        Phase::Lost => (
            None,
            Color {
                red: 248,
                green: 112,
                blue: 108,
                alpha: 255,
            },
        ),
    };
    let label = match state.phase {
        Phase::Ready => "PRESS START",
        Phase::Playing { .. } => "PLAYING",
        Phase::Won => "YOU WON",
        Phase::Lost => "GAME OVER",
    };
    hud::draw(&state.font, label, status, &mut arena, &mut commands)
        .expect("frame command budget includes the HUD");
    if let Some(ball) = ball {
        rect(
            &mut commands,
            Point::new(ball.x() - BALL_HALF, ball.y() - BALL_HALF),
            Size::new(BALL_HALF * 2.0, BALL_HALF * 2.0),
            Color::WHITE,
        );
        if matches!(state.phase, Phase::Ready) {
            let aim = Vec2::new(0.0, -48.0).rotate(Radians::new(0.35));
            commands
                .push(RenderCommand::DrawLine {
                    start: ball,
                    end: point(Vec2::new(ball.x(), ball.y()) + aim),
                    width: StrokeWidth::new(2.0),
                    color: status,
                })
                .expect("frame command budget includes the aim line");
        }
    } else {
        rect(
            &mut commands,
            Point::new(480.0, 352.0),
            Size::new(320.0, 16.0),
            status,
        );
    }
    let frame = Frame::new(
        Canvas::new(1280, 720),
        Color {
            red: 16,
            green: 23,
            blue: 34,
            alpha: 255,
        },
        commands.finish(),
    );
    let uploads = if state.font_uploaded {
        &[][..]
    } else {
        &arena.push_slice(
            1,
            TextureUpload {
                id: TextureId::new(0),
                image: hud::image(),
            },
        )[..]
    };
    FrameOutput::new(frame, uploads)
}

fn draw_court(state: &GameState, commands: &mut RenderCommands<'_>) {
    let wall = Color {
        red: 73,
        green: 87,
        blue: 108,
        alpha: 255,
    };
    for (x, y, width, height) in [
        (16.0, 64.0, 1248.0, 8.0),
        (16.0, TOP, 8.0, 632.0),
        (RIGHT, TOP, 8.0, 632.0),
    ] {
        rect(commands, Point::new(x, y), Size::new(width, height), wall);
    }
    for (_, brick) in state.bricks.iter() {
        rect(
            commands,
            Point::new(
                brick.center.x() - BRICK_HALF.x,
                brick.center.y() - BRICK_HALF.y,
            ),
            Size::new(BRICK_HALF.x * 2.0, BRICK_HALF.y * 2.0),
            brick.color,
        );
    }
    rect(
        commands,
        Point::new(state.paddle.x() - PADDLE_HALF, PADDLE_Y),
        Size::new(PADDLE_HALF * 2.0, 14.0),
        Color {
            red: 227,
            green: 238,
            blue: 245,
            alpha: 255,
        },
    );
    for life in 0..state.lives {
        rect(
            commands,
            Point::new(28.0 + f32::from(life) * 24.0, 28.0),
            Size::new(14.0, 14.0),
            Color {
                red: 112,
                green: 230,
                blue: 204,
                alpha: 255,
            },
        );
    }
}

fn rect(commands: &mut RenderCommands<'_>, origin: Point, size: Size, color: Color) {
    commands
        .push(RenderCommand::DrawRect {
            rect: Rect::new(origin, size),
            style: RectStyle::Filled(color),
        })
        .expect("frame command budget includes court rectangles");
}

fn point(value: Vec2) -> Point {
    Point::new(value.x, value.y)
}
