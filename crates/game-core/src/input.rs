use core::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameInput {
    pub frame_duration: Duration,
    pub controller: ControllerInput,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControllerInput {
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

impl ControllerInput {
    pub fn start_frame(&mut self) {
        for button in self.buttons_mut() {
            button.half_transitions = 0;
        }
    }

    pub fn release_all(&mut self) {
        for button in self.buttons_mut() {
            button.record(ButtonPosition::Up);
        }
    }

    fn buttons_mut(&mut self) -> [&mut ButtonState; 12] {
        let Self {
            move_up,
            move_down,
            move_left,
            move_right,
            action_up,
            action_down,
            action_left,
            action_right,
            left_shoulder,
            right_shoulder,
            back,
            start,
        } = self;
        [
            move_up,
            move_down,
            move_left,
            move_right,
            action_up,
            action_down,
            action_left,
            action_right,
            left_shoulder,
            right_shoulder,
            back,
            start,
        ]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonPosition {
    #[default]
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonState {
    ended: ButtonPosition,
    half_transitions: u32,
}

impl ButtonState {
    #[must_use]
    pub fn ended(self) -> ButtonPosition {
        self.ended
    }

    #[must_use]
    pub fn half_transitions(self) -> u32 {
        self.half_transitions
    }

    pub fn record(&mut self, position: ButtonPosition) {
        if self.ended != position {
            self.ended = position;
            self.half_transitions = self.half_transitions.saturating_add(1);
        }
    }
}
