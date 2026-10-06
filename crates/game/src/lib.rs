#![no_std]

use game_core::asset::read_rgba;
use game_core::font::{Font, MAX_FONT_FILE_BYTES};
use game_core::render::RenderCommands;
use game_memory::Arena;
use platform_api::input::{ButtonPosition, ButtonState, FrameInput};
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::render::{
    Camera, Canvas, CanvasRect, Color, Frame, GlyphQuad, Image, PixelRect, Point, Radians, Rect,
    RectStyle, RenderCommand, Sampling, Scale, Size, Sprite, StrokeWidth, TextureId, TextureUpload,
    Transform,
};
use platform_api::services::{PlatformApi, StartupError};

const IMAGE_BUDGET: usize = 8 + 512 * 512 * 4;
const COMMAND_BUDGET: usize = 12_000;
const GLYPH_BUDGET: usize = 4096;

pub struct GameState {
    font: Font,
    pan: Point,
    zoom: f32,
    phase: f32,
    scene: Scene,
}

enum Scene {
    Demo,
    Stress,
}

pub fn initialize<'p, 't>(
    persistent: PersistentMemory<'p>,
    transient: TransientMemory<'t>,
    api: &PlatformApi,
) -> Result<(&'p mut GameState, [TextureUpload<'t>; 2]), StartupError> {
    let PersistentMemory { bytes: persistent } = persistent;
    let TransientMemory { bytes: transient } = transient;
    let mut persistent = Arena::new(persistent);
    let mut transient = Arena::new(transient);
    let sprites = load(&mut transient, api, "assets/sprites.rgba", IMAGE_BUDGET)?;
    let atlas = load(&mut transient, api, "assets/font.rgba", IMAGE_BUDGET)?;
    let metadata = load(&mut transient, api, "assets/font.gfn", MAX_FONT_FILE_BYTES)?;
    let sprites = image(sprites, "assets/sprites.rgba")?;
    if sprites.width() != 128 || sprites.height() != 32 {
        return Err(StartupError::InvalidAsset {
            path: "assets/sprites.rgba",
        });
    }
    let atlas = image(atlas, "assets/font.rgba")?;
    let font = Font::from_file(metadata, TextureId::new(1), atlas).map_err(|_| {
        StartupError::InvalidAsset {
            path: "assets/font.gfn",
        }
    })?;
    let state = persistent.push(GameState {
        font,
        pan: Point::new(0.0, 0.0),
        zoom: 1.0,
        phase: 0.0,
        scene: Scene::Demo,
    });
    Ok((
        state,
        [
            TextureUpload {
                id: TextureId::new(0),
                image: sprites,
            },
            TextureUpload {
                id: TextureId::new(1),
                image: atlas,
            },
        ],
    ))
}

fn load<'a>(
    arena: &mut Arena<'a>,
    api: &PlatformApi,
    path: &'static str,
    budget: usize,
) -> Result<&'a [u8], StartupError> {
    let buffer = arena.push_slice(budget, 0_u8);
    let len =
        (api.load_entire_file)(path, buffer).map_err(|error| StartupError::File { path, error })?;
    Ok(&buffer[..len])
}

fn image<'a>(bytes: &'a [u8], path: &'static str) -> Result<Image<'a>, StartupError> {
    read_rgba(bytes).map_err(|_| StartupError::InvalidAsset { path })
}

fn advance(state: &mut GameState, input: &FrameInput) {
    let dt = input.dt.seconds();
    let buttons = input.controller;
    state.pan = Point::new(
        (state.pan.x() + (held(buttons.move_right) - held(buttons.move_left)) * dt * 220.0)
            .clamp(-2000.0, 2000.0),
        (state.pan.y() + (held(buttons.move_down) - held(buttons.move_up)) * dt * 220.0)
            .clamp(-2000.0, 2000.0),
    );
    state.zoom = (state.zoom + (held(buttons.right_shoulder) - held(buttons.left_shoulder)) * dt)
        .clamp(0.5, 2.0);
    state.phase = (state.phase + dt * 0.7) % core::f32::consts::TAU;
    let transitions = buttons.start.half_transitions();
    let presses = transitions / 2
        + u32::from(transitions % 2 == 1 && buttons.start.ended() == ButtonPosition::Down);
    if presses % 2 == 1 {
        state.scene = match state.scene {
            Scene::Demo => Scene::Stress,
            Scene::Stress => Scene::Demo,
        };
    }
}

pub fn update<'a>(
    state: &mut GameState,
    memory: TransientMemory<'a>,
    input: &FrameInput,
) -> Frame<'a> {
    advance(state, input);
    let TransientMemory { bytes } = memory;
    let mut transient = Arena::new(bytes);
    let storage = transient.push_slice(COMMAND_BUDGET, RenderCommand::SetCamera(Camera::default()));
    let glyphs = transient.push_slice(
        GLYPH_BUDGET,
        GlyphQuad {
            source: PixelRect::new(0, 0, 0, 0),
            transform: Transform {
                center: Point::new(0.0, 0.0),
                size: Size::new(0.0, 0.0),
                rotation: Radians::new(0.0),
            },
        },
    );
    let mut commands = RenderCommands::new(storage, glyphs);
    commands.push(RenderCommand::SetClip(Some(CanvasRect::new(rect(
        40.0, 150.0, 1200.0, 430.0,
    )))));
    commands.push(RenderCommand::SetCamera(Camera {
        origin: state.pan,
        zoom: Scale::new(state.zoom),
    }));
    match state.scene {
        Scene::Demo => demo(&mut commands, state),
        Scene::Stress => {
            for row in 0_u16..100 {
                for column in 0_u16..100 {
                    sprite(
                        &mut commands,
                        Point::new(
                            46.0 + f32::from(column) * 12.0,
                            154.0 + f32::from(row) * 4.2,
                        ),
                        12.0,
                        column % 4,
                        state.phase,
                        Sampling::Nearest,
                        Color::WHITE,
                    );
                }
            }
            for row in 0_u16..25 {
                text(
                    &mut commands,
                    &state.font,
                    "AVATAR gypsy AVATAR gypsy AVATAR gypsy AV!",
                    Point::new(48.0, 150.0 + f32::from(row) * 17.0),
                    0.5,
                    Color::WHITE,
                );
            }
        }
    }
    commands.push(RenderCommand::SetCamera(Camera::default()));
    commands.push(RenderCommand::SetClip(None));
    let title = "GAMZ / THE LITTLE ATLAS";
    let width = state.font.measure(title, Scale::new(1.0)).width();
    text(
        &mut commands,
        &state.font,
        title,
        Point::new((1280.0 - width) * 0.5, 30.0),
        1.0,
        Color(233, 239, 245, 255),
    );
    text(
        &mut commands,
        &state.font,
        "WASD pan  /  Q E zoom  /  Space demo or 10,000 sprites  /  Esc exit",
        Point::new(150.0, 90.0),
        0.7,
        Color(150, 169, 190, 255),
    );
    text(
        &mut commands,
        &state.font,
        "AVATAR, To, Wa: proportional spacing & kerning\nQuiet gypsies jump below the baseline.",
        Point::new(70.0, 614.0),
        0.75,
        Color(185, 202, 218, 255),
    );
    Frame {
        canvas: Canvas::new(1280, 720),
        clear: Color(15, 22, 33, 255),
        commands: commands.finish(),
    }
}

fn held(button: ButtonState) -> f32 {
    if button.ended() == ButtonPosition::Down {
        1.0
    } else {
        0.0
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), Size::new(width, height))
}

fn text(
    commands: &mut RenderCommands<'_>,
    font: &Font,
    value: &str,
    position: Point,
    scale: f32,
    color: Color,
) {
    commands.draw_text(
        font,
        value,
        position,
        Scale::new(scale),
        color,
        Sampling::Linear,
    );
}

fn sprite(
    commands: &mut RenderCommands<'_>,
    center: Point,
    size: f32,
    tile: u16,
    rotation: f32,
    sampling: Sampling,
    tint: Color,
) {
    commands.push(RenderCommand::DrawSprite(Sprite {
        texture: TextureId::new(0),
        source: PixelRect::new(tile * 32, 0, 32, 32),
        transform: Transform {
            center,
            size: Size::new(size, size),
            rotation: Radians::new(rotation),
        },
        tint,
        sampling,
    }));
}

fn demo(commands: &mut RenderCommands<'_>, state: &GameState) {
    commands.push(RenderCommand::DrawRect {
        rect: rect(40.0, 150.0, 1200.0, 430.0),
        style: RectStyle::Filled(Color(24, 35, 48, 255)),
    });
    for row in 0_u16..6 {
        for column in 0_u16..12 {
            sprite(
                commands,
                Point::new(
                    85.0 + f32::from(column) * 48.0,
                    215.0 + f32::from(row) * 48.0,
                ),
                46.0,
                (row + column) % 2,
                0.0,
                Sampling::Nearest,
                Color::WHITE,
            );
        }
    }
    sprite(
        commands,
        Point::new(260.0, 320.0),
        82.0,
        2,
        state.phase,
        Sampling::Nearest,
        Color::WHITE,
    );
    sprite(
        commands,
        Point::new(305.0, 355.0),
        82.0,
        3,
        -state.phase,
        Sampling::Linear,
        Color(150, 220, 255, 170),
    );
    text(
        commands,
        &state.font,
        "THE GARDEN / nearest",
        Point::new(75.0, 158.0),
        0.65,
        Color(190, 222, 210, 255),
    );
    sampling_panel(commands, &state.font);
}

fn sampling_panel(commands: &mut RenderCommands<'_>, font: &Font) {
    text(
        commands,
        font,
        "SAMPLING / same atlas",
        Point::new(725.0, 174.0),
        0.65,
        Color(236, 207, 154, 255),
    );
    sprite(
        commands,
        Point::new(810.0, 295.0),
        130.0,
        2,
        0.15,
        Sampling::Nearest,
        Color::WHITE,
    );
    sprite(
        commands,
        Point::new(1050.0, 295.0),
        130.0,
        2,
        0.15,
        Sampling::Linear,
        Color::WHITE,
    );
    text(
        commands,
        font,
        "NEAREST",
        Point::new(750.0, 377.0),
        0.6,
        Color::WHITE,
    );
    text(
        commands,
        font,
        "LINEAR",
        Point::new(1000.0, 377.0),
        0.6,
        Color::WHITE,
    );
    commands.push(RenderCommand::DrawRect {
        rect: rect(710.0, 430.0, 220.0, 95.0),
        style: RectStyle::Filled(Color(74, 121, 160, 160)),
    });
    commands.push(RenderCommand::DrawRect {
        rect: rect(820.0, 470.0, 240.0, 80.0),
        style: RectStyle::Outline {
            color: Color(239, 181, 86, 255),
            width: StrokeWidth::new(4.0),
        },
    });
    commands.push(RenderCommand::DrawLine {
        start: Point::new(1000.0, 520.0),
        end: Point::new(1320.0, 610.0),
        width: StrokeWidth::new(12.0),
        color: Color(143, 195, 178, 220),
    });
    text(
        commands,
        font,
        "ALPHA / OUTLINE / CLIP",
        Point::new(720.0, 550.0),
        0.55,
        Color(185, 202, 218, 255),
    );
}
