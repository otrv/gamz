use platform_api::input::{ButtonPosition, ButtonState};

pub(crate) fn released_button() -> ButtonState {
    ButtonState {
        ended: ButtonPosition::Up,
        half_transitions: 0,
    }
}
