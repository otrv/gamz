#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use game_entry::SelectedGame;
use platform_api::Game;
use platform_api::audio::{AudioBuffer, SampleRate, StereoFrame};
use platform_api::input::{ButtonPosition, ButtonState, ControllerInput, FrameInput};
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::services::{FileError, MAX_FILE_BYTES, PlatformApi};
use platform_api::timing::DeltaTime;
use renderer::wgpu::Renderer;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::HtmlCanvasElement;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::{Window, WindowId};

const PERSISTENT_MEMORY_BYTES: usize = 64 * 1024;
const TRANSIENT_MEMORY_BYTES: usize = 256 * 1024 * 1024;
const AUDIO_FRAMES: usize = 4096;

#[wasm_bindgen(module = "/src/host.js")]
extern "C" {
    fn prepare() -> js_sys::Promise;
    fn ready();
    fn fail(message: &str);
    fn active() -> bool;
    fn next_frame() -> js_sys::Promise;
    fn sample_rate() -> u32;
    fn needs_audio() -> bool;
    #[wasm_bindgen(catch)]
    fn play_audio(left: &[f32], right: &[f32]) -> Result<(), JsValue>;
    fn asset(path: &str) -> Option<js_sys::Uint8Array>;
}

struct Platform {
    canvas: Option<HtmlCanvasElement>,
    controller: Rc<RefCell<ControllerInput>>,
}

impl ApplicationHandler for Platform {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(canvas) = self.canvas.take() else {
            return;
        };
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_canvas(Some(canvas))
                .with_prevent_default(true),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                fail(&error.to_string());
                event_loop.exit();
                return;
            }
        };
        let controller = Rc::clone(&self.controller);
        spawn_local(async move {
            if let Err(error) = Box::pin(run::<SelectedGame>(window, controller)).await {
                let message = error
                    .as_string()
                    .or_else(|| {
                        error
                            .dyn_ref::<js_sys::Error>()
                            .map(|error| String::from(error.message()))
                    })
                    .unwrap_or_else(|| "Unable to run the game.".into());
                fail(&message);
            }
        });
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Focused(false) => self.controller.borrow_mut().release_all(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        repeat: false,
                        ..
                    },
                ..
            } if active() => {
                if let Some(button) = controller_button(&mut self.controller.borrow_mut(), key) {
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

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(|info| fail(&info.to_string())));
    let event_loop = EventLoop::new().map_err(js_error)?;
    event_loop.spawn_app(Platform {
        canvas: Some(canvas),
        controller: Rc::new(RefCell::new(ControllerInput::default())),
    });
    Ok(())
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

async fn run<G: Game>(
    window: Arc<Window>,
    controller: Rc<RefCell<ControllerInput>>,
) -> Result<(), JsValue> {
    JsFuture::from(prepare()).await?;
    let mut persistent = Vec::<u8>::new();
    persistent
        .try_reserve_exact(PERSISTENT_MEMORY_BYTES)
        .map_err(js_error)?;
    let mut transient = Vec::<u8>::new();
    transient
        .try_reserve_exact(TRANSIENT_MEMORY_BYTES)
        .map_err(js_error)?;
    let transient = &mut transient.spare_capacity_mut()[..TRANSIENT_MEMORY_BYTES];
    let mut size = window.inner_size();
    let mut renderer = Renderer::new(Arc::clone(&window), size.width, size.height)
        .await
        .map_err(js_error)?;
    let state = {
        let (state, uploads) = G::initialize(
            PersistentMemory {
                bytes: &mut persistent.spare_capacity_mut()[..PERSISTENT_MEMORY_BYTES],
            },
            TransientMemory { bytes: transient },
            &PlatformApi { load_entire_file },
        )
        .map_err(js_error)?;
        for &upload in uploads.as_ref() {
            renderer.upload_texture(upload).map_err(js_error)?;
        }
        state
    };
    let mut samples = [StereoFrame::default(); AUDIO_FRAMES];
    let mut left = [0.0; AUDIO_FRAMES];
    let mut right = [0.0; AUDIO_FRAMES];
    let mut previous = None;
    let mut input = FrameInput {
        dt: DeltaTime::from_elapsed(Duration::ZERO),
        controller: ControllerInput::default(),
    };
    ready();
    loop {
        let frame = state.update(TransientMemory { bytes: transient }, &input);
        let mut redraw = renderer.draw(&frame).map_err(js_error)?.is_none();
        if active() && needs_audio() {
            let rate = SampleRate::from_hz(sample_rate())
                .ok_or_else(|| js_error("unsupported audio sample rate"))?;
            state.update_audio(AudioBuffer::new(rate, &mut samples));
            for ((sample, left), right) in samples.iter().zip(&mut left).zip(&mut right) {
                *left = f32::from(sample.left) / 32768.0;
                *right = f32::from(sample.right) / 32768.0;
            }
            play_audio(&left, &right)?;
        }
        loop {
            let timestamp = JsFuture::from(next_frame())
                .await?
                .as_f64()
                .ok_or_else(|| js_error("invalid animation timestamp"))?;
            let current = window.inner_size();
            if current != size {
                size = current;
                renderer.resize(size.width, size.height).map_err(js_error)?;
                redraw = true;
            }
            if redraw {
                redraw = renderer.draw(&frame).map_err(js_error)?.is_none();
            }
            if !active() {
                previous = None;
                let mut controller = controller.borrow_mut();
                controller.release_all();
                controller.start_frame();
                continue;
            }
            let elapsed = previous.map_or(0.0, |last| (timestamp - last) / 1000.0_f64);
            previous = Some(timestamp);
            let mut controller = controller.borrow_mut();
            input = FrameInput {
                dt: DeltaTime::from_elapsed(
                    Duration::try_from_secs_f64(elapsed.clamp(0.0, 0.05)).map_err(js_error)?,
                ),
                controller: *controller,
            };
            controller.start_frame();
            break;
        }
    }
}

fn load_entire_file(path: &str, destination: &mut [u8]) -> Result<usize, FileError> {
    if path.is_empty()
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || path.contains(['\\', ':', '\0'])
    {
        return Err(FileError::InvalidPath);
    }
    let bytes = asset(path).ok_or(FileError::NotFound)?;
    let length = usize::try_from(bytes.length()).unwrap();
    if length > MAX_FILE_BYTES || length > destination.len() {
        return Err(FileError::TooLarge);
    }
    bytes.copy_to(&mut destination[..length]);
    Ok(length)
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
