use std::collections::TryReserveError;
use std::fmt;
use std::num::{NonZeroU32, NonZeroU64};
use std::time::{Duration, Instant};

use game_core::input::{ButtonPosition, ButtonState, ControllerInput, FrameInput};
use game_memory::{Arena, GameMemory, PersistentMemory};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowId};

const PERSISTENT_MEMORY_BYTES: usize = 64 * 1024 * 1024;
const TRANSIENT_MEMORY_BYTES: usize = 256 * 1024 * 1024;
const WINDOW_TITLE: &str = "gamz";
const WINDOW_SIZE: LogicalSize<f64> = LogicalSize::new(1280.0, 720.0);
const FALLBACK_REFRESH_MILLIHERTZ: NonZeroU32 = NonZeroU32::new(60_000).unwrap();
const NANOS_PER_CYCLE_AT_ONE_MILLIHERTZ: u64 = 1_000_000_000_000;

#[derive(Debug)]
pub(crate) enum PlatformError {
    Memory(TryReserveError),
    EventLoop(EventLoopError),
    Window(OsError),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Memory(error) => write!(f, "cannot reserve game memory: {error}"),
            Self::EventLoop(error) => write!(f, "cannot start the event loop: {error}"),
            Self::Window(error) => write!(f, "cannot open the window: {error}"),
        }
    }
}

enum WindowState {
    Opening,
    Open {
        window: Window,
        frame_duration: Option<Duration>,
    },
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
            Ok(window) => WindowState::Open {
                window,
                frame_duration: None,
            },
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
            WindowEvent::Moved(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let WindowState::Open { frame_duration, .. } = &mut self.window {
                    *frame_duration = None;
                }
            }
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
    let mut reservation = Vec::<u8>::new();
    reservation
        .try_reserve_exact(PERSISTENT_MEMORY_BYTES + TRANSIENT_MEMORY_BYTES)
        .map_err(PlatformError::Memory)?;
    let (persistent_bytes, transient_bytes) = reservation.spare_capacity_mut()
        [..PERSISTENT_MEMORY_BYTES + TRANSIENT_MEMORY_BYTES]
        .split_at_mut(PERSISTENT_MEMORY_BYTES);
    let mut persistent = PersistentMemory::new(persistent_bytes);
    let mut event_loop = EventLoop::new().map_err(PlatformError::EventLoop)?;
    let mut platform = Platform {
        window: WindowState::Opening,
        controller: ControllerInput::default(),
    };
    let mut next_frame = Instant::now();
    loop {
        let timeout = next_frame.saturating_duration_since(Instant::now());
        if let PumpStatus::Exit(code) = event_loop.pump_app_events(Some(timeout), &mut platform) {
            return match (platform.window, code) {
                (WindowState::Failed(error), _) => Err(PlatformError::Window(error)),
                (WindowState::Opening | WindowState::Open { .. }, 0) => Ok(()),
                (WindowState::Opening | WindowState::Open { .. }, code) => {
                    Err(PlatformError::EventLoop(EventLoopError::ExitFailure(code)))
                }
            };
        }
        let WindowState::Open {
            window,
            frame_duration,
        } = &mut platform.window
        else {
            continue;
        };
        let frame_duration = *frame_duration.get_or_insert_with(|| monitor_frame_duration(window));
        let now = Instant::now();
        if now < next_frame {
            continue;
        }
        let input = FrameInput {
            frame_duration,
            controller: platform.controller,
        };
        platform.controller.start_frame();
        game::update(
            GameMemory {
                persistent: persistent.reborrow(),
                transient: Arena::new(transient_bytes),
            },
            &input,
        );
        next_frame = (next_frame + frame_duration).max(now);
    }
}

fn monitor_frame_duration(window: &Window) -> Duration {
    let refresh_millihertz = window
        .current_monitor()
        .and_then(|monitor| monitor.refresh_rate_millihertz())
        .and_then(NonZeroU32::new)
        .unwrap_or(FALLBACK_REFRESH_MILLIHERTZ);
    Duration::from_nanos(NANOS_PER_CYCLE_AT_ONE_MILLIHERTZ / NonZeroU64::from(refresh_millihertz))
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
