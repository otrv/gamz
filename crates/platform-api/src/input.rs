use core::time::Duration;

use math::Vec2;

use crate::file::IoStatus;
use crate::request::RequestId;
use crate::window::{WindowState, WindowStatus};

pub struct Input<'a> {
    pub elapsed: Duration,
    pub actions: Actions,
    pub pointer: Option<Vec2>,
    pub scroll: Vec2,
    pub text: &'a str,
    pub window: WindowState,
    pub events: &'a [PlatformEvent<'a>],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Actions {
    pub movement: Vec2,
    pub aim: Vec2,
    pub move_up: ButtonState,
    pub move_down: ButtonState,
    pub move_left: ButtonState,
    pub move_right: ButtonState,
    pub action_up: ButtonState,
    pub action_down: ButtonState,
    pub action_left: ButtonState,
    pub action_right: ButtonState,
    pub left_shoulder: ButtonState,
    pub right_shoulder: ButtonState,
    pub back: ButtonState,
    pub start: ButtonState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonPosition {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonState {
    pub ended: ButtonPosition,
    pub half_transitions: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformEvent<'a> {
    ReadCompleted {
        id: RequestId,
        bytes: &'a [u8],
        status: IoStatus,
    },
    WriteCompleted {
        id: RequestId,
        bytes: usize,
        status: IoStatus,
    },
    WindowModeCompleted {
        id: RequestId,
        status: WindowStatus,
    },
    CloseRequested,
    Suspended,
    Resumed,
}
