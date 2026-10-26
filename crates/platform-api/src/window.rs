#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowMode {
    Windowed,
    Fullscreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub focused: bool,
    pub mode: WindowMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowStatus {
    Applied,
    Denied,
    Cancelled,
}
