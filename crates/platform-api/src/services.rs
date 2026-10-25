use core::fmt;

use crate::memory::MemoryRange;

pub const MAX_REQUESTS: usize = 8;
pub const MAX_EVENTS: usize = MAX_REQUESTS + 8;
pub const MAX_PATH_BYTES: usize = 256;
pub const MAX_IO_BYTES: usize = 1024 * 1024;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy)]
pub struct WindowTitle {
    bytes: [u8; 128],
    len: usize,
}

impl WindowTitle {
    pub fn new(title: &str) -> Result<Self, RequestError> {
        if title.len() > 128 || title.contains('\0') {
            return Err(RequestError::InvalidValue);
        }
        let mut value = Self {
            bytes: [0; 128],
            len: title.len(),
        };
        value.bytes[..title.len()].copy_from_slice(title.as_bytes());
        Ok(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).expect("title was constructed from UTF-8")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestId(u64);

impl RequestId {
    #[must_use]
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct FilePath {
    bytes: [u8; MAX_PATH_BYTES],
    len: usize,
}

impl FilePath {
    pub fn new(path: &str) -> Result<Self, RequestError> {
        if path.is_empty()
            || path.len() > MAX_PATH_BYTES
            || path.contains(['\\', ':', '\0', '?', '#', '%'])
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(RequestError::InvalidPath);
        }
        let mut result = Self {
            bytes: [0; MAX_PATH_BYTES],
            len: path.len(),
        };
        result.bytes[..path.len()].copy_from_slice(path.as_bytes());
        Ok(result)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).expect("path was constructed from UTF-8")
    }
}

#[derive(Clone, Copy)]
pub struct ReadRequest {
    pub path: FilePath,
    pub offset: u64,
    pub destination: MemoryRange,
}

#[derive(Clone, Copy)]
pub enum WriteMode {
    Replace,
    AtOffset(u64),
}

#[derive(Clone, Copy)]
pub struct WriteRequest {
    pub path: FilePath,
    pub source: MemoryRange,
    pub mode: WriteMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestError {
    QueueFull,
    InvalidPath,
    InvalidRange,
    MemoryBusy,
    InvalidValue,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileError {
    NotFound,
    PermissionDenied,
    TooLarge,
    Io,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IoStatus {
    Complete,
    EndOfFile,
    Cancelled,
    Failed(FileError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelStatus {
    Requested,
    TooLate,
    UnknownRequest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowMode {
    Windowed,
    Fullscreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorMode {
    Visible,
    Hidden,
}

#[derive(Clone, Copy)]
pub enum WindowChange {
    Size { width: u32, height: u32 },
    Mode(WindowMode),
    Cursor(CursorMode),
    Title(WindowTitle),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub focused: bool,
    pub mode: WindowMode,
    pub cursor: CursorMode,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            framebuffer_width: 1280,
            framebuffer_height: 720,
            focused: true,
            mode: WindowMode::Windowed,
            cursor: CursorMode::Visible,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowStatus {
    Applied,
    Denied,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformEvent {
    ReadCompleted {
        id: RequestId,
        bytes: usize,
        status: IoStatus,
    },
    WriteCompleted {
        id: RequestId,
        bytes: usize,
        status: IoStatus,
    },
    WindowCompleted {
        id: RequestId,
        status: WindowStatus,
    },
    CloseRequested,
    Suspended,
    Resumed,
}

#[derive(Clone, Copy, Debug)]
pub enum StartupError {
    InsufficientPersistentMemory { required: usize, available: usize },
    InsufficientTransientMemory { required: usize, available: usize },
    Game { code: u32 },
}

impl fmt::Display for StartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientPersistentMemory {
                required,
                available,
            } => write!(
                f,
                "insufficient persistent game memory: {available} bytes available, {required} required"
            ),
            Self::InsufficientTransientMemory {
                required,
                available,
            } => write!(
                f,
                "insufficient transient game memory: {available} bytes available, {required} required"
            ),
            Self::Game { code } => write!(f, "game initialization failed: {code}"),
        }
    }
}

impl core::error::Error for StartupError {}
