#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Radians(f32);

impl Radians {
    #[must_use]
    pub fn new(value: f32) -> Self {
        assert!(value.is_finite());
        Self(value)
    }

    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }
}
