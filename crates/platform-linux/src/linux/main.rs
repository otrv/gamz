#[cfg(not(target_os = "linux"))]
compile_error!("the platform layer is implemented only for Linux");

mod audio;
#[path = "../controls.rs"]
mod controls;
#[path = "../files.rs"]
mod files;
mod input;
#[path = "../limits.rs"]
mod limits;
#[path = "../memory.rs"]
mod memory;
#[path = "../window.rs"]
mod window;

use std::collections::TryReserveError;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use platform_api::input::{Input, PlatformEvent};
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::render::Frame;
use platform_api::window::{WindowMode, WindowState};
use platform_api::{FrameError, Game, Platform, StartupError};
use renderer::wgpu::{Limits, Renderer, RendererError};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Fullscreen, Window, WindowId};

use audio::Audio;
use files::{Completions, Files};
use input::InputState;
use limits::{
    MAX_COMMANDS, MAX_EVENTS, MAX_QUADS, MAX_TEXTURE_BYTES, MAX_TEXTURE_SIDE, MAX_TEXTURES,
};
use memory::{PERSISTENT_MEMORY_BYTES, TRANSIENT_MEMORY_BYTES};
use window::ModeRequests;

const WINDOW_TITLE: &str = "gamz";
const WINDOW_SIZE: LogicalSize<f64> = LogicalSize::new(1280.0, 720.0);
const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000_u64.div_ceil(60));

#[derive(Debug)]
enum PlatformError {
    Memory(TryReserveError),
    EventLoop(EventLoopError),
    Window(OsError),
    Renderer(RendererError),
    Startup(StartupError),
    Frame(FrameError),
    Audio(Box<dyn std::error::Error>),
    Input(&'static str),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Memory(error) => write!(f, "cannot reserve game memory: {error}"),
            Self::EventLoop(error) => write!(f, "cannot start the event loop: {error}"),
            Self::Window(error) => write!(f, "cannot open the window: {error}"),
            Self::Renderer(error) => write!(f, "renderer: {error}"),
            Self::Startup(error) => write!(f, "game initialization: {error:?}"),
            Self::Frame(error) => write!(f, "game frame: {error:?}"),
            Self::Audio(error) => write!(f, "audio: {error}"),
            Self::Input(error) => write!(f, "input: {error}"),
        }
    }
}

enum WindowProgress {
    Opening,
    Open(Arc<Window>),
    Failed(OsError),
}

struct Application {
    window: WindowProgress,
    input: InputState,
    close_requested: bool,
    lifecycle: Option<PlatformEvent<'static>>,
    input_error: Option<&'static str>,
}

struct Context {
    files: Files,
    modes: ModeRequests,
    renderer: Renderer,
    exit: bool,
}

impl Context {
    fn try_new(window: &Arc<Window>) -> Result<Self, RendererError> {
        let size = window.inner_size();
        let renderer = pollster::block_on(Renderer::new(
            Arc::clone(window),
            size.width,
            size.height,
            Limits {
                commands: MAX_COMMANDS,
                quads: MAX_QUADS,
                textures: MAX_TEXTURES,
                texture_side: MAX_TEXTURE_SIDE,
                texture_bytes: MAX_TEXTURE_BYTES,
            },
        ))?;
        Ok(Self {
            files: Files::new(),
            modes: ModeRequests::new(),
            renderer,
            exit: false,
        })
    }

    fn api(&mut self) -> Platform<'_, Self> {
        Platform {
            context: self,
            read_file: |context, request| context.files.read(request),
            write_file: |context, request| context.files.write(request),
            cancel: |context, id| {
                if id.0 % 2 == 0 {
                    context.modes.cancel(id)
                } else {
                    context.files.cancel(id)
                }
            },
            upload_texture: |context, upload| context.renderer.upload_texture(upload),
            release_texture: |context, id| context.renderer.release_texture(id),
            set_window_mode: |context, mode| context.modes.request(mode),
            exit: |context| context.exit = true,
        }
    }
}

fn window_state(window: &Window) -> WindowState {
    let size = window.inner_size();
    let logical = size.to_logical::<u32>(window.scale_factor());
    WindowState {
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
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.lifecycle = Some(PlatformEvent::Resumed);
        if !matches!(self.window, WindowProgress::Opening) {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(WINDOW_SIZE);
        self.window = match event_loop.create_window(attributes) {
            Ok(window) => {
                window.set_ime_allowed(true);
                WindowProgress::Open(Arc::new(window))
            }
            Err(error) => {
                event_loop.exit();
                WindowProgress::Failed(error)
            }
        };
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.lifecycle = Some(PlatformEvent::Suspended);
        if let Err(error) = self.input.event(&WindowEvent::Focused(false), 1.0) {
            self.input_error = Some(error);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let scale = match &self.window {
            WindowProgress::Open(window) => window.scale_factor(),
            WindowProgress::Opening | WindowProgress::Failed(_) => 1.0,
        };
        if let Err(error) = self.input.event(&event, scale) {
            self.input_error = Some(error);
            event_loop.exit();
            return;
        }
        match event {
            WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => self.close_requested = true,
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
    let mut transient = Vec::<u8>::new();
    transient
        .try_reserve_exact(TRANSIENT_MEMORY_BYTES)
        .map_err(PlatformError::Memory)?;
    let mut event_loop = EventLoop::new().map_err(PlatformError::EventLoop)?;
    let Some(mut application) = open_window(&mut event_loop)? else {
        return Ok(());
    };
    let WindowProgress::Open(window) = &application.window else {
        return exit_result(application.window, 0);
    };
    let window = Arc::clone(window);
    let mut size = window.inner_size();
    let mut context = Context::try_new(&window).map_err(PlatformError::Renderer)?;
    let state = game::GameState::initialize(
        PersistentMemory {
            bytes: &mut persistent.spare_capacity_mut()[..PERSISTENT_MEMORY_BYTES],
        },
        TRANSIENT_MEMORY_BYTES,
        &mut context.api(),
    )
    .map_err(PlatformError::Startup)?;
    if context.exit {
        return Ok(());
    }
    let mut audio = Audio::start(state).map_err(PlatformError::Audio)?;
    let mut completions = Completions::new();
    let mut previous_frame = Instant::now();
    let mut next_frame = previous_frame;
    let mut next_stats = previous_frame;
    while !context.exit {
        let timeout = next_frame.saturating_duration_since(Instant::now());
        if let PumpStatus::Exit(code) = event_loop.pump_app_events(Some(timeout), &mut application)
        {
            if let Some(error) = application.input_error {
                return Err(PlatformError::Input(error));
            }
            return exit_result(application.window, code);
        }
        if Instant::now() < next_frame {
            continue;
        }
        let current_size = window.inner_size();
        if size != current_size {
            size = current_size;
            context
                .renderer
                .resize(size.width, size.height)
                .map_err(PlatformError::Renderer)?;
        }
        let now = Instant::now();
        let window_state = window_state(&window);
        context.modes.apply(window_state.mode, |mode| {
            window.set_fullscreen(match mode {
                WindowMode::Windowed => None,
                WindowMode::Fullscreen => Some(Fullscreen::Borderless(None)),
            });
        });
        let frame = update_game(
            &mut application,
            &mut context,
            state,
            TransientMemory {
                bytes: &mut transient.spare_capacity_mut()[..TRANSIENT_MEMORY_BYTES],
            },
            &mut completions,
            window_state,
            now.duration_since(previous_frame),
        )
        .map_err(PlatformError::Frame)?;
        if context.exit {
            break;
        }
        audio
            .update(state)
            .map_err(|error| PlatformError::Audio(error.into()))?;
        window.pre_present_notify();
        if let Some(stats) = context
            .renderer
            .draw(&frame)
            .map_err(PlatformError::Renderer)?
            && now >= next_stats
        {
            println!(
                "commands={} quads={} batches={} vertex_bytes={}",
                stats.commands, stats.quads, stats.batches, stats.vertex_bytes
            );
            next_stats = now + Duration::from_secs(1);
        }
        previous_frame = now;
        next_frame = now + FRAME_INTERVAL;
    }
    Ok(())
}

fn update_game<'frame>(
    application: &mut Application,
    context: &mut Context,
    state: &mut game::GameState,
    memory: TransientMemory<'frame>,
    completions: &mut Completions,
    window: WindowState,
    elapsed: Duration,
) -> Result<Frame<'frame>, FrameError> {
    context.files.collect(completions);
    let mut events = [PlatformEvent::Resumed; MAX_EVENTS];
    let mut count = completions.events(&mut events);
    count += context.modes.events(&mut events[count..]);
    if let Some(event) = application.lifecycle.take() {
        events[count] = event;
        count += 1;
    }
    if core::mem::take(&mut application.close_requested) {
        events[count] = PlatformEvent::CloseRequested;
        count += 1;
    }
    let input = Input {
        elapsed,
        actions: application.input.actions,
        pointer: application.input.pointer,
        scroll: application.input.scroll,
        text: &application.input.text,
        window,
        events: &events[..count],
    };
    let frame = state.update(&input, memory, &mut context.api())?;
    application.input.finish_frame();
    context.files.recycle(completions);
    Ok(frame)
}

fn open_window(event_loop: &mut EventLoop<()>) -> Result<Option<Application>, PlatformError> {
    let mut application = Application {
        window: WindowProgress::Opening,
        input: InputState::new(),
        close_requested: false,
        lifecycle: None,
        input_error: None,
    };
    while matches!(application.window, WindowProgress::Opening) {
        if let PumpStatus::Exit(code) =
            event_loop.pump_app_events(Some(Duration::from_millis(16)), &mut application)
        {
            if let Some(error) = application.input_error {
                return Err(PlatformError::Input(error));
            }
            return exit_result(application.window, code).map(|()| None);
        }
    }
    Ok(Some(application))
}

fn exit_result(window: WindowProgress, code: i32) -> Result<(), PlatformError> {
    match (window, code) {
        (WindowProgress::Failed(error), _) => Err(PlatformError::Window(error)),
        (_, 0) => Ok(()),
        (_, code) => Err(PlatformError::EventLoop(EventLoopError::ExitFailure(code))),
    }
}
