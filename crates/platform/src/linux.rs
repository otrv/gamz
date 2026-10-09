mod audio;
mod services;

use std::collections::TryReserveError;
use std::fmt;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use platform_api::input::{ButtonPosition, ButtonState, ControllerInput, FrameInput};
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::services::{PlatformApi, StartupError};
use platform_api::timing::DeltaTime;
use renderer_wgpu::{Renderer, RendererError};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowId};

use crate::memory::{PERSISTENT_MEMORY_BYTES, TRANSIENT_MEMORY_BYTES};
use audio::Audio;
use services::load_entire_file;

const WINDOW_TITLE: &str = "gamz";
const WINDOW_SIZE: LogicalSize<f64> = LogicalSize::new(1280.0, 720.0);
const MAX_FRAMES_PER_SECOND: NonZeroU32 = NonZeroU32::new(60).unwrap();

#[derive(Debug)]
pub(crate) enum PlatformError {
    Memory(TryReserveError),
    EventLoop(EventLoopError),
    Window(OsError),
    Renderer(RendererError),
    Startup(StartupError),
    Audio(Box<dyn std::error::Error>),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Memory(error) => write!(f, "cannot reserve game memory: {error}"),
            Self::EventLoop(error) => write!(f, "cannot start the event loop: {error}"),
            Self::Window(error) => write!(f, "cannot open the window: {error}"),
            Self::Renderer(error) => write!(f, "renderer: {error}"),
            Self::Startup(error) => write!(f, "game initialization: {error}"),
            Self::Audio(error) => write!(f, "audio: {error}"),
        }
    }
}

enum WindowState {
    Opening,
    Open(Arc<Window>),
    Failed(OsError),
}

struct Platform {
    window: WindowState,
    controller: ControllerInput,
}

impl ApplicationHandler for Platform {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !matches!(self.window, WindowState::Opening) {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(WINDOW_SIZE);
        self.window = match event_loop.create_window(attributes) {
            Ok(window) => WindowState::Open(Arc::new(window)),
            Err(error) => {
                event_loop.exit();
                WindowState::Failed(error)
            }
        };
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::Focused(false) => self.controller.release_all(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                if key == KeyCode::Escape && state == ElementState::Pressed {
                    event_loop.exit();
                }
                if let Some(button) = controller_button(&mut self.controller, key) {
                    button.record(match state {
                        ElementState::Pressed => ButtonPosition::Down,
                        ElementState::Released => ButtonPosition::Up,
                    });
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn run() -> Result<(), PlatformError> {
    let mut persistent = Vec::<u8>::new();
    persistent
        .try_reserve_exact(PERSISTENT_MEMORY_BYTES)
        .map_err(PlatformError::Memory)?;
    let mut reservation = Vec::<u8>::new();
    reservation
        .try_reserve_exact(TRANSIENT_MEMORY_BYTES)
        .map_err(PlatformError::Memory)?;
    let transient_bytes = &mut reservation.spare_capacity_mut()[..TRANSIENT_MEMORY_BYTES];
    let mut event_loop = EventLoop::new().map_err(PlatformError::EventLoop)?;
    let mut platform = Platform {
        window: WindowState::Opening,
        controller: ControllerInput::default(),
    };
    while matches!(platform.window, WindowState::Opening) {
        if let PumpStatus::Exit(code) =
            event_loop.pump_app_events(Some(Duration::from_millis(16)), &mut platform)
        {
            return exit_result(platform.window, code);
        }
    }
    let WindowState::Open(window) = &platform.window else {
        return exit_result(platform.window, 0);
    };
    let mut size = window.inner_size();
    let mut renderer =
        pollster::block_on(Renderer::new(Arc::clone(window), size.width, size.height))
            .map_err(PlatformError::Renderer)?;
    let state = {
        let (state, uploads) = game::initialize(
            PersistentMemory {
                bytes: &mut persistent.spare_capacity_mut()[..PERSISTENT_MEMORY_BYTES],
            },
            TransientMemory {
                bytes: transient_bytes,
            },
            &PlatformApi { load_entire_file },
        )
        .map_err(PlatformError::Startup)?;
        for upload in uploads {
            renderer
                .upload_texture(upload)
                .map_err(PlatformError::Renderer)?;
        }
        state
    };
    let mut audio = Audio::start(state).map_err(PlatformError::Audio)?;
    let frame_interval =
        Duration::from_nanos(1_000_000_000_u64.div_ceil(u64::from(MAX_FRAMES_PER_SECOND.get())));
    let mut previous_frame = Instant::now();
    let mut next_frame = previous_frame + frame_interval;
    let mut next_stats = next_frame;
    loop {
        let timeout = next_frame.saturating_duration_since(Instant::now());
        if let PumpStatus::Exit(code) = event_loop.pump_app_events(Some(timeout), &mut platform) {
            return exit_result(platform.window, code);
        }
        let WindowState::Open(window) = &platform.window else {
            continue;
        };
        if Instant::now() < next_frame {
            continue;
        }
        let current_size = window.inner_size();
        if size != current_size {
            size = current_size;
            renderer
                .resize(size.width, size.height)
                .map_err(PlatformError::Renderer)?;
        }
        let now = Instant::now();
        let input = FrameInput {
            dt: DeltaTime::from_elapsed(now.duration_since(previous_frame)),
            controller: platform.controller,
        };
        previous_frame = now;
        platform.controller.start_frame();
        let frame = game::update(
            state,
            TransientMemory {
                bytes: transient_bytes,
            },
            &input,
        );
        audio
            .update(state)
            .map_err(|error| PlatformError::Audio(error.into()))?;
        window.pre_present_notify();
        if let Some(stats) = renderer.draw(&frame).map_err(PlatformError::Renderer)?
            && now >= next_stats
        {
            println!(
                "commands={} quads={} batches={} vertex_bytes={}",
                stats.commands, stats.quads, stats.batches, stats.vertex_bytes
            );
            next_stats = now + Duration::from_secs(1);
        }
        next_frame = now + frame_interval;
    }
}

fn exit_result(window: WindowState, code: i32) -> Result<(), PlatformError> {
    match (window, code) {
        (WindowState::Failed(error), _) => Err(PlatformError::Window(error)),
        (_, 0) => Ok(()),
        (_, code) => Err(PlatformError::EventLoop(EventLoopError::ExitFailure(code))),
    }
}

fn controller_button(controller: &mut ControllerInput, key: KeyCode) -> Option<&mut ButtonState> {
    Some(match key {
        KeyCode::KeyW => &mut controller.move_up,
        KeyCode::KeyS => &mut controller.move_down,
        KeyCode::KeyA => &mut controller.move_left,
        KeyCode::KeyD => &mut controller.move_right,
        KeyCode::ArrowUp => &mut controller.action_up,
        KeyCode::ArrowDown => &mut controller.action_down,
        KeyCode::ArrowLeft => &mut controller.action_left,
        KeyCode::ArrowRight => &mut controller.action_right,
        KeyCode::KeyQ => &mut controller.left_shoulder,
        KeyCode::KeyE => &mut controller.right_shoulder,
        KeyCode::Escape => &mut controller.back,
        KeyCode::Space => &mut controller.start,
        _ => return None,
    })
}
