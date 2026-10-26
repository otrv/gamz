use crate::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub origin: Vec2,
    pub size: Vec2,
}
