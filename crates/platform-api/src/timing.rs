use core::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeltaTime(Duration);

impl DeltaTime {
    #[must_use]
    pub fn from_elapsed(elapsed: Duration) -> Self {
        Self(elapsed)
    }

    #[must_use]
    pub fn seconds(self) -> f32 {
        self.0.as_secs_f32()
    }
}
