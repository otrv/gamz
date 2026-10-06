use core::fmt;

pub const MAX_FILE_BYTES: usize = 16 * 1024 * 1024 + 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileError {
    InvalidPath,
    NotFound,
    PermissionDenied,
    TooLarge,
    Unsupported,
    Io,
}

pub struct PlatformApi {
    pub load_entire_file: fn(&str, &mut [u8]) -> Result<usize, FileError>,
}

#[derive(Debug)]
pub enum StartupError {
    InsufficientPersistentMemory {
        required: usize,
        available: usize,
    },
    File {
        path: &'static str,
        error: FileError,
    },
    InvalidAsset {
        path: &'static str,
    },
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
            Self::File { path, error } => write!(f, "{path}: file load failed: {error:?}"),
            Self::InvalidAsset { path } => write!(f, "{path}: invalid asset"),
        }
    }
}
