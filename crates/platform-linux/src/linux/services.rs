use std::fs::File;
use std::io::{self, Read};
use std::path::{Component, Path};

use platform_api::services::{FileError, MAX_FILE_BYTES};
use rustix::fs::{Mode, OFlags};

const MAX_PATH_BYTES: usize = 256;
const MAX_READ_ATTEMPTS: usize = 4096;

pub(crate) fn load_entire_file(path: &str, output: &mut [u8]) -> Result<usize, FileError> {
    if path.is_empty()
        || path.len() > MAX_PATH_BYTES
        || path.as_bytes().contains(&0)
        || !Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(FileError::InvalidPath);
    }
    let descriptor = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| file_error(io::Error::from(error).kind()))?;
    let mut file = File::from(descriptor);
    let metadata = file.metadata().map_err(|error| file_error(error.kind()))?;
    if !metadata.is_file() {
        return Err(FileError::Unsupported);
    }
    let capacity = output.len().min(MAX_FILE_BYTES);
    if metadata.len() > u64::try_from(capacity).unwrap() {
        return Err(FileError::TooLarge);
    }
    let mut len = 0;
    for _ in 0..MAX_READ_ATTEMPTS {
        let mut extra = [0_u8; 1];
        let at_capacity = len == capacity;
        let destination = if at_capacity {
            &mut extra[..]
        } else {
            &mut output[len..capacity]
        };
        match file.read(destination) {
            Ok(0) => return Ok(len),
            Ok(_) if at_capacity => return Err(FileError::TooLarge),
            Ok(count) => len += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(file_error(error.kind())),
        }
    }
    Err(FileError::Io)
}

fn file_error(kind: io::ErrorKind) -> FileError {
    match kind {
        io::ErrorKind::NotFound => FileError::NotFound,
        io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
        _ => FileError::Io,
    }
}
