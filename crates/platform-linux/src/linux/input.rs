use math::Vec2;
use platform_api::input::{Actions, ButtonPosition, ButtonState};
use winit::event::{DeviceId, ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::controls::released_button;
use crate::limits::MAX_TEXT_BYTES;

const MAX_INPUT_DEVICES: usize = 16;
const MAX_SCROLL: f32 = 1_000_000.0;
const MOVE_UP: u16 = 1 << 0;
const MOVE_DOWN: u16 = 1 << 1;
const MOVE_LEFT: u16 = 1 << 2;
const MOVE_RIGHT: u16 = 1 << 3;
const ACTION_UP: u16 = 1 << 4;
const ACTION_DOWN: u16 = 1 << 5;
const ACTION_LEFT: u16 = 1 << 6;
const ACTION_RIGHT: u16 = 1 << 7;
const LEFT_SHOULDER: u16 = 1 << 8;
const RIGHT_SHOULDER: u16 = 1 << 9;
const BACK: u16 = 1 << 10;
const START: u16 = 1 << 11;

pub(super) struct InputState {
    pub(super) actions: Actions,
    pub(super) pointer: Option<Vec2>,
    pub(super) scroll: Vec2,
    pub(super) text: String,
    devices: [Option<Device>; MAX_INPUT_DEVICES],
    focused: bool,
    ime_composing: bool,
}

#[derive(Clone, Copy)]
struct Device {
    id: DeviceId,
    keyboard: u16,
    mouse: u16,
}

#[derive(Clone, Copy)]
enum Source {
    Keyboard,
    Mouse,
}

impl InputState {
    pub(super) fn new() -> Self {
        Self {
            actions: empty_actions(),
            pointer: None,
            scroll: Vec2::ZERO,
            text: String::with_capacity(MAX_TEXT_BYTES),
            devices: [None; MAX_INPUT_DEVICES],
            focused: true,
            ime_composing: false,
        }
    }

    pub(super) fn event(
        &mut self,
        event: &WindowEvent,
        scale_factor: f64,
    ) -> Result<(), &'static str> {
        if let WindowEvent::Focused(focused) = event {
            self.focused = *focused;
            if !focused {
                self.devices.fill(None);
                self.pointer = None;
                self.scroll = Vec2::ZERO;
                self.text.clear();
                self.ime_composing = false;
                self.refresh_actions()?;
            }
        }
        if !self.focused {
            return Ok(());
        }
        if !scale_factor.is_finite() || scale_factor <= 0.0 {
            return Err("input scale factor must be finite and positive");
        }
        match event {
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                if !event.repeat
                    && let PhysicalKey::Code(key) = event.physical_key
                    && let Some(mask) = key_mask(key)
                {
                    self.record(*device_id, Source::Keyboard, mask, event.state)?;
                }
                if event.state == ElementState::Pressed
                    && !is_synthetic
                    && !self.ime_composing
                    && let Some(text) = &event.text
                {
                    self.append_text(text)?;
                }
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                let mask = match button {
                    MouseButton::Left => ACTION_DOWN,
                    MouseButton::Right => ACTION_RIGHT,
                    _ => return Ok(()),
                };
                self.record(*device_id, Source::Mouse, mask, *state)?;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let logical = position.to_logical::<f32>(scale_factor);
                if !logical.x.is_finite() || !logical.y.is_finite() {
                    return Err("pointer coordinates must be finite");
                }
                self.pointer = Some(Vec2::new(logical.x, logical.y));
            }
            WindowEvent::CursorLeft { .. } => self.pointer = None,
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(*x, *y),
                    MouseScrollDelta::PixelDelta(position) => {
                        let logical = position.to_logical::<f32>(scale_factor);
                        Vec2::new(logical.x, logical.y)
                    }
                };
                if !delta.x.is_finite() || !delta.y.is_finite() {
                    return Err("scroll components must be finite");
                }
                self.scroll.x = accumulate_scroll(self.scroll.x, delta.x);
                self.scroll.y = accumulate_scroll(self.scroll.y, delta.y);
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Enabled | Ime::Disabled => self.ime_composing = false,
                Ime::Commit(text) => {
                    self.ime_composing = false;
                    self.append_text(text)?;
                }
                Ime::Preedit(text, _) => self.ime_composing = !text.is_empty(),
            },
            _ => {}
        }
        Ok(())
    }

    pub(super) fn finish_frame(&mut self) {
        for button in buttons_mut(&mut self.actions) {
            button.half_transitions = 0;
        }
        self.scroll = Vec2::ZERO;
        self.text.clear();
    }

    fn record(
        &mut self,
        id: DeviceId,
        source: Source,
        mask: u16,
        state: ElementState,
    ) -> Result<(), &'static str> {
        let existing = self
            .devices
            .iter()
            .position(|device| device.is_some_and(|device| device.id == id));
        let slot = if let Some(index) = existing {
            &mut self.devices[index]
        } else if state == ElementState::Released {
            return Ok(());
        } else {
            self.devices
                .iter_mut()
                .find(|device| device.is_none())
                .ok_or("input device capacity exceeded")?
        };
        let device = slot.get_or_insert(Device {
            id,
            keyboard: 0,
            mouse: 0,
        });
        let held = match source {
            Source::Keyboard => &mut device.keyboard,
            Source::Mouse => &mut device.mouse,
        };
        match state {
            ElementState::Pressed => *held |= mask,
            ElementState::Released => *held &= !mask,
        }
        if device.keyboard == 0 && device.mouse == 0 {
            *slot = None;
        }
        self.refresh_actions()
    }

    fn refresh_actions(&mut self) -> Result<(), &'static str> {
        let mut keyboard = 0;
        let mut mouse = 0;
        for device in self.devices.iter().flatten() {
            keyboard |= device.keyboard;
            mouse |= device.mouse;
        }
        let mut actions = self.actions;
        for (index, button) in buttons_mut(&mut actions).into_iter().enumerate() {
            let ended = if (keyboard | mouse) & (1 << index) == 0 {
                ButtonPosition::Up
            } else {
                ButtonPosition::Down
            };
            if button.ended != ended {
                button.half_transitions = button
                    .half_transitions
                    .checked_add(1)
                    .ok_or("input transition count exceeded")?;
                button.ended = ended;
            }
        }
        actions.movement = Vec2::new(
            direction(keyboard, MOVE_RIGHT, MOVE_LEFT),
            direction(keyboard, MOVE_DOWN, MOVE_UP),
        );
        actions.aim = Vec2::new(
            direction(keyboard, ACTION_RIGHT, ACTION_LEFT),
            direction(keyboard, ACTION_DOWN, ACTION_UP),
        );
        self.actions = actions;
        Ok(())
    }

    fn append_text(&mut self, text: &str) -> Result<(), &'static str> {
        if text.len() > MAX_TEXT_BYTES - self.text.len() {
            return Err("input text capacity exceeded");
        }
        self.text.push_str(text);
        Ok(())
    }
}

fn empty_actions() -> Actions {
    Actions {
        movement: Vec2::ZERO,
        aim: Vec2::ZERO,
        move_up: released_button(),
        move_down: released_button(),
        move_left: released_button(),
        move_right: released_button(),
        action_up: released_button(),
        action_down: released_button(),
        action_left: released_button(),
        action_right: released_button(),
        left_shoulder: released_button(),
        right_shoulder: released_button(),
        back: released_button(),
        start: released_button(),
    }
}

fn buttons_mut(actions: &mut Actions) -> [&mut ButtonState; 12] {
    [
        &mut actions.move_up,
        &mut actions.move_down,
        &mut actions.move_left,
        &mut actions.move_right,
        &mut actions.action_up,
        &mut actions.action_down,
        &mut actions.action_left,
        &mut actions.action_right,
        &mut actions.left_shoulder,
        &mut actions.right_shoulder,
        &mut actions.back,
        &mut actions.start,
    ]
}

fn key_mask(key: KeyCode) -> Option<u16> {
    Some(match key {
        KeyCode::KeyW => MOVE_UP,
        KeyCode::KeyS => MOVE_DOWN,
        KeyCode::KeyA => MOVE_LEFT,
        KeyCode::KeyD => MOVE_RIGHT,
        KeyCode::ArrowUp => ACTION_UP,
        KeyCode::ArrowDown => ACTION_DOWN,
        KeyCode::ArrowLeft => ACTION_LEFT,
        KeyCode::ArrowRight => ACTION_RIGHT,
        KeyCode::KeyQ => LEFT_SHOULDER,
        KeyCode::KeyE => RIGHT_SHOULDER,
        KeyCode::Escape => BACK,
        KeyCode::Space => START,
        _ => return None,
    })
}

fn direction(held: u16, positive: u16, negative: u16) -> f32 {
    f32::from(u8::from(held & positive != 0)) - f32::from(u8::from(held & negative != 0))
}

fn accumulate_scroll(current: f32, delta: f32) -> f32 {
    (current + delta.clamp(-MAX_SCROLL, MAX_SCROLL)).clamp(-MAX_SCROLL, MAX_SCROLL)
}
