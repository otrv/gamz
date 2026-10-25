#[cfg(not(target_os = "linux"))]
compile_error!("the platform layer is implemented only for Linux");

mod audio;
#[path = "../memory.rs"]
mod memory;
mod services;

use std::collections::TryReserveError;
use std::fmt;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use platform_api::Game;
use platform_api::input::{ButtonPosition, ButtonState, ControllerInput, FrameInput};
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::render::FrameOutput;
use platform_api::services::{
    self as contract, CursorMode, MAX_EVENTS, MAX_REQUESTS, PlatformEvent, RequestId, StartupError,
    WindowChange, WindowMode, WindowStatus,
};
use platform_api::timing::DeltaTime;
use renderer::wgpu::{Renderer, RendererError};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Fullscreen, Window, WindowId};

use crate::memory::{PERSISTENT_MEMORY_BYTES, TRANSIENT_MEMORY_BYTES};
use audio::Audio;
use services::Services;

const WINDOW_TITLE: &str = "gamz";
const WINDOW_SIZE: LogicalSize<f64> = LogicalSize::new(1280.0, 720.0);
const MAX_FRAMES_PER_SECOND: NonZeroU32 = NonZeroU32::new(60).unwrap();

#[derive(Debug)]
enum PlatformError {
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
    close_requested: bool,
    window_requests: [Option<WindowRequest>; MAX_REQUESTS],
}

#[derive(Clone, Copy)]
struct WindowRequest {
    id: RequestId,
    change: WindowChange,
    deadline: Instant,
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
            WindowEvent::CloseRequested => self.close_requested = true,
            WindowEvent::Destroyed => event_loop.exit(),
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
                    self.close_requested = true;
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

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gamz: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), PlatformError> {
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
    let Some(mut platform) = open_window(&mut event_loop)? else {
        return Ok(());
    };
    let WindowState::Open(window) = &platform.window else {
        return exit_result(platform.window, 0);
    };
    let mut size = window.inner_size();
    let mut renderer =
        pollster::block_on(Renderer::new(Arc::clone(window), size.width, size.height))
            .map_err(PlatformError::Renderer)?;
    let mut services = Services::new();
    let state = game::GameState::initialize(
        PersistentMemory {
            bytes: &mut persistent.spare_capacity_mut()[..PERSISTENT_MEMORY_BYTES],
        },
        transient_bytes.len(),
        &mut services,
    )
    .map_err(PlatformError::Startup)?;
    if services.exit_requested() {
        return Ok(());
    }
    let mut audio = Audio::start(state, &mut services).map_err(PlatformError::Audio)?;
    let mut events = [PlatformEvent::Resumed; MAX_EVENTS];
    let frame_interval =
        Duration::from_nanos(1_000_000_000_u64.div_ceil(u64::from(MAX_FRAMES_PER_SECOND.get())));
    let mut previous_frame = Instant::now();
    let mut next_frame = previous_frame + frame_interval;
    let mut next_stats = next_frame;
    while !services.exit_requested() {
        let timeout = next_frame.saturating_duration_since(Instant::now());
        if let PumpStatus::Exit(code) = event_loop.pump_app_events(Some(timeout), &mut platform) {
            return exit_result(platform.window, code);
        }
        let WindowState::Open(window) = &platform.window else {
            continue;
        };
        apply_window_requests(&mut services, window, &mut platform.window_requests);
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
        let dt = DeltaTime::from_elapsed(now.duration_since(previous_frame));
        let controller = platform.controller;
        previous_frame = now;
        platform.controller.start_frame();
        if core::mem::take(&mut platform.close_requested) {
            services.request_close();
        }
        let count = services.poll_events(&mut events);
        let input = FrameInput::new(dt, controller, services.window_state(), &events[..count]);
        let frame = state.update(
            &mut services,
            TransientMemory {
                bytes: transient_bytes,
            },
            &input,
        );
        if services.exit_requested() {
            return Ok(());
        }
        audio
            .update(state, &mut services)
            .map_err(|error| PlatformError::Audio(error.into()))?;
        if services.exit_requested() {
            return Ok(());
        }
        present(&mut renderer, window, &frame, now, &mut next_stats)?;
        next_frame = now + frame_interval;
    }
    Ok(())
}

fn open_window(event_loop: &mut EventLoop<()>) -> Result<Option<Platform>, PlatformError> {
    let mut platform = Platform {
        window: WindowState::Opening,
        controller: ControllerInput::default(),
        close_requested: false,
        window_requests: [None; MAX_REQUESTS],
    };
    while matches!(platform.window, WindowState::Opening) {
        if let PumpStatus::Exit(code) =
            event_loop.pump_app_events(Some(Duration::from_millis(16)), &mut platform)
        {
            return exit_result(platform.window, code).map(|()| None);
        }
    }
    Ok(Some(platform))
}

fn present(
    renderer: &mut Renderer,
    window: &Window,
    frame: &FrameOutput<'_>,
    now: Instant,
    next_stats: &mut Instant,
) -> Result<(), PlatformError> {
    renderer
        .upload_textures(frame.uploads())
        .map_err(PlatformError::Renderer)?;
    window.pre_present_notify();
    if let Some(stats) = renderer
        .draw(&frame.frame)
        .map_err(PlatformError::Renderer)?
        && now >= *next_stats
    {
        println!(
            "commands={} quads={} batches={} vertex_bytes={}",
            stats.commands, stats.quads, stats.batches, stats.vertex_bytes
        );
        *next_stats = now + Duration::from_secs(1);
    }
    Ok(())
}

fn apply_window_requests(
    services: &mut Services<'_>,
    window: &Window,
    pending: &mut [Option<WindowRequest>; MAX_REQUESTS],
) {
    let now = Instant::now();
    let mut state = services.window_state();
    for slot in &mut *pending {
        if slot.is_some() {
            continue;
        }
        let Some((id, change)) = services.take_window_request() else {
            break;
        };
        match change {
            WindowChange::Size { width, height } => {
                let _ = window.request_inner_size(LogicalSize::new(width, height));
                *slot = Some(WindowRequest {
                    id,
                    change,
                    deadline: now + Duration::from_secs(2),
                });
            }
            WindowChange::Mode(mode) => {
                window.set_fullscreen(match mode {
                    WindowMode::Windowed => None,
                    WindowMode::Fullscreen => Some(Fullscreen::Borderless(None)),
                });
                *slot = Some(WindowRequest {
                    id,
                    change,
                    deadline: now + Duration::from_secs(2),
                });
            }
            WindowChange::Cursor(cursor) => {
                window.set_cursor_visible(cursor == CursorMode::Visible);
                state.cursor = cursor;
                services.complete_window(id, WindowStatus::Applied);
            }
            WindowChange::Title(title) => {
                window.set_title(title.as_str());
                services.complete_window(id, WindowStatus::Applied);
            }
        }
    }
    let size = window.inner_size();
    let logical = size.to_logical::<u32>(window.scale_factor());
    state = contract::WindowState {
        width: logical.width,
        height: logical.height,
        framebuffer_width: size.width,
        framebuffer_height: size.height,
        focused: window.has_focus(),
        mode: if window.fullscreen().is_some() {
            WindowMode::Fullscreen
        } else {
            WindowMode::Windowed
        },
        cursor: state.cursor,
    };
    for slot in pending {
        let Some(request) = *slot else { continue };
        let applied = match request.change {
            WindowChange::Size { width, height } => state.width == width && state.height == height,
            WindowChange::Mode(mode) => state.mode == mode,
            WindowChange::Cursor(_) | WindowChange::Title(_) => unreachable!(),
        };
        if applied || now >= request.deadline {
            services.complete_window(
                request.id,
                if applied {
                    WindowStatus::Applied
                } else {
                    WindowStatus::Denied
                },
            );
            *slot = None;
        }
    }
    services.set_window_state(state);
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
