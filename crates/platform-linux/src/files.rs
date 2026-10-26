use std::array;
use std::collections::VecDeque;
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, TryLockError};
use std::thread::{self, JoinHandle};

use platform_api::file::{FileError, IoStatus, ReadRequest, WriteMode, WriteRequest};
use platform_api::input::PlatformEvent;
use platform_api::request::{CancelStatus, RequestError, RequestId};
use rustix::fs::{AtFlags, FileType, Mode, OFlags};

use crate::limits::{MAX_FILE_BYTES, MAX_IO_BYTES, MAX_PATH_BYTES, MAX_REQUESTS};

const ACTIVE: u8 = 0;
const CANCELLED: u8 = 1;
const FINISHED: u8 = 2;
const MAX_IO_ATTEMPTS: usize = 4096;
const IO_CHUNK_BYTES: usize = 64 * 1024;
const MAX_TEMP_ATTEMPTS: usize = 16;

struct FilePath {
    bytes: [u8; MAX_PATH_BYTES],
    len: usize,
}

impl FilePath {
    fn try_new(path: &str) -> Result<Self, RequestError> {
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

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).expect("path was constructed from UTF-8")
    }
}

enum Operation {
    Read {
        path: FilePath,
        offset: u64,
        len: usize,
    },
    Write {
        path: FilePath,
        mode: WriteMode,
        len: usize,
    },
}

struct Job {
    slot: usize,
    id: RequestId,
    operation: Operation,
    buffer: Vec<u8>,
}

struct Transfer {
    bytes: usize,
    status: IoStatus,
}

struct Completion {
    job: Job,
    transfer: Transfer,
}

struct Queue {
    pending: VecDeque<Job>,
    completed: VecDeque<Completion>,
    stopping: bool,
}

struct Worker {
    queue: Mutex<Queue>,
    wake: Condvar,
    progress: [AtomicU8; MAX_REQUESTS],
}

pub(crate) struct Files {
    worker: Arc<Worker>,
    thread: Option<JoinHandle<()>>,
    buffers: [Option<Vec<u8>>; MAX_REQUESTS],
    slots: [Option<RequestId>; MAX_REQUESTS],
    next_request: u64,
}

pub(crate) struct Completions {
    completed: VecDeque<Completion>,
}

impl Completions {
    pub(crate) fn new() -> Self {
        Self {
            completed: VecDeque::with_capacity(MAX_REQUESTS),
        }
    }

    pub(crate) fn events<'a>(&'a self, output: &mut [PlatformEvent<'a>]) -> usize {
        assert!(output.len() >= self.completed.len());
        for (completion, event) in self.completed.iter().zip(output) {
            let Completion { job, transfer } = completion;
            *event = match job.operation {
                Operation::Read { len, .. } => {
                    assert!(transfer.bytes <= len);
                    PlatformEvent::ReadCompleted {
                        id: job.id,
                        bytes: &job.buffer[..transfer.bytes],
                        status: transfer.status,
                    }
                }
                Operation::Write { len, .. } => {
                    assert!(transfer.bytes <= len);
                    PlatformEvent::WriteCompleted {
                        id: job.id,
                        bytes: transfer.bytes,
                        status: transfer.status,
                    }
                }
            };
        }
        self.completed.len()
    }
}

impl Files {
    pub(crate) fn new() -> Self {
        let worker = Arc::new(Worker {
            queue: Mutex::new(Queue {
                pending: VecDeque::with_capacity(MAX_REQUESTS),
                completed: VecDeque::with_capacity(MAX_REQUESTS),
                stopping: false,
            }),
            wake: Condvar::new(),
            progress: array::from_fn(|_| AtomicU8::new(FINISHED)),
        });
        let buffers = array::from_fn(|_| Some(vec![0; MAX_IO_BYTES]));
        let thread_worker = Arc::clone(&worker);
        let thread = thread::spawn(move || thread_worker.run());
        Self {
            worker,
            thread: Some(thread),
            buffers,
            slots: [None; MAX_REQUESTS],
            next_request: 1,
        }
    }

    fn next_slot(&mut self) -> Result<(usize, RequestId), RequestError> {
        let slot = self
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(RequestError::QueueFull)?;
        let id = RequestId(self.next_request);
        self.next_request = self
            .next_request
            .checked_add(2)
            .ok_or(RequestError::QueueFull)?;
        Ok((slot, id))
    }

    fn submit(&mut self, operation: Operation, input: &[u8]) -> Result<RequestId, RequestError> {
        let (slot, id) = self.next_slot()?;
        let mut queue = self.worker.queue.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => RequestError::QueueFull,
            TryLockError::Poisoned(_) => RequestError::Unavailable,
        })?;
        if queue.stopping || self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
            return Err(RequestError::Unavailable);
        }
        if queue.pending.len() == MAX_REQUESTS {
            return Err(RequestError::QueueFull);
        }
        let mut buffer = self.buffers[slot]
            .take()
            .expect("idle slot retains its staging buffer");
        buffer[..input.len()].copy_from_slice(input);
        self.worker.progress[slot].store(ACTIVE, Ordering::Release);
        self.slots[slot] = Some(id);
        queue.pending.push_back(Job {
            slot,
            id,
            operation,
            buffer,
        });
        drop(queue);
        self.worker.wake.notify_one();
        Ok(id)
    }

    pub(crate) fn read(&mut self, request: ReadRequest<'_>) -> Result<RequestId, RequestError> {
        let path = FilePath::try_new(request.path)?;
        check_io_range(request.bytes, request.offset)?;
        self.submit(
            Operation::Read {
                path,
                offset: request.offset,
                len: request.bytes,
            },
            &[],
        )
    }

    pub(crate) fn write(&mut self, request: WriteRequest<'_>) -> Result<RequestId, RequestError> {
        let path = FilePath::try_new(request.path)?;
        let offset = match request.mode {
            WriteMode::Replace => 0,
            WriteMode::AtOffset(offset) => offset,
        };
        check_io_range(request.bytes.len(), offset)?;
        self.submit(
            Operation::Write {
                path,
                mode: request.mode,
                len: request.bytes.len(),
            },
            request.bytes,
        )
    }

    pub(crate) fn cancel(&mut self, id: RequestId) -> CancelStatus {
        for (index, slot) in self.slots.iter().enumerate() {
            if *slot == Some(id) {
                return match self.worker.progress[index].compare_exchange(
                    ACTIVE,
                    CANCELLED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) | Err(CANCELLED) => CancelStatus::Requested,
                    Err(_) => CancelStatus::TooLate,
                };
            }
        }
        CancelStatus::UnknownRequest
    }

    pub(crate) fn collect(&mut self, completions: &mut Completions) {
        assert!(
            completions.completed.is_empty(),
            "recycle the previous batch first"
        );
        match self.worker.queue.try_lock() {
            Ok(mut queue) => std::mem::swap(&mut queue.completed, &mut completions.completed),
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Poisoned(_)) => panic!("file worker queue poisoned"),
        }
    }

    pub(crate) fn recycle(&mut self, completions: &mut Completions) {
        for Completion { job, .. } in completions.completed.drain(..) {
            assert_eq!(self.slots[job.slot], Some(job.id));
            assert!(self.buffers[job.slot].is_none());
            self.buffers[job.slot] = Some(job.buffer);
            self.slots[job.slot] = None;
        }
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        for progress in &self.worker.progress {
            let _ =
                progress.compare_exchange(ACTIVE, CANCELLED, Ordering::AcqRel, Ordering::Acquire);
        }
        if let Ok(mut queue) = self.worker.queue.lock() {
            queue.stopping = true;
        }
        self.worker.wake.notify_one();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Worker {
    fn run(&self) {
        loop {
            let job = {
                let mut queue = self.queue.lock().expect("file worker queue poisoned");
                if queue.pending.is_empty() && !queue.stopping {
                    queue = self.wake.wait(queue).expect("file worker queue poisoned");
                }
                if queue.stopping {
                    return;
                }
                queue.pending.pop_front()
            };
            let Some(mut job) = job else { continue };
            let progress = &self.progress[job.slot];
            let transfer = if progress.load(Ordering::Acquire) == CANCELLED {
                Transfer {
                    bytes: 0,
                    status: IoStatus::Cancelled,
                }
            } else {
                match perform(&mut job, progress) {
                    Ok(transfer) => transfer,
                    Err(error) => Transfer {
                        bytes: 0,
                        status: IoStatus::Failed(file_error(error.kind())),
                    },
                }
            };
            let mut queue = self.queue.lock().expect("file worker queue poisoned");
            progress.store(FINISHED, Ordering::Release);
            assert!(queue.completed.len() < MAX_REQUESTS);
            queue.completed.push_back(Completion { job, transfer });
        }
    }
}

fn check_io_range(len: usize, offset: u64) -> Result<(), RequestError> {
    if len > MAX_IO_BYTES {
        return Err(RequestError::InvalidRange);
    }
    offset
        .checked_add(u64::try_from(len).map_err(|_| RequestError::InvalidRange)?)
        .filter(|&end| end <= MAX_FILE_BYTES)
        .ok_or(RequestError::InvalidRange)?;
    Ok(())
}

fn perform(job: &mut Job, progress: &AtomicU8) -> io::Result<Transfer> {
    match &job.operation {
        Operation::Read { path, offset, len } => {
            let file = open_regular(path.as_str(), OFlags::RDONLY)?;
            Ok(read(&file, *offset, &mut job.buffer[..*len], progress))
        }
        Operation::Write {
            path,
            mode: WriteMode::AtOffset(offset),
            len,
        } => {
            let file = open_regular(path.as_str(), OFlags::WRONLY | OFlags::CREATE)?;
            Ok(write(&file, *offset, &job.buffer[..*len], progress))
        }
        Operation::Write {
            path,
            mode: WriteMode::Replace,
            len,
        } => replace(path, job.id, &job.buffer[..*len], progress),
    }
}

fn open_regular(path: &str, flags: OFlags) -> io::Result<File> {
    let descriptor = rustix::fs::open(
        path,
        flags | OFlags::NONBLOCK | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::from_raw_mode(0o666),
    )?;
    let file = File::from(descriptor);
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::ErrorKind::Unsupported.into());
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(io::ErrorKind::FileTooLarge.into());
    }
    Ok(file)
}

fn read(file: &File, offset: u64, output: &mut [u8], progress: &AtomicU8) -> Transfer {
    let mut bytes = 0;
    for _ in 0..MAX_IO_ATTEMPTS {
        let status = if progress.load(Ordering::Acquire) == CANCELLED {
            Some(IoStatus::Cancelled)
        } else if bytes == output.len() {
            Some(IoStatus::Complete)
        } else {
            let end = output.len().min(bytes + IO_CHUNK_BYTES);
            match file.read_at(
                &mut output[bytes..end],
                offset + u64::try_from(bytes).expect("bounded transfer"),
            ) {
                Ok(0) => Some(IoStatus::EndOfFile),
                Ok(count) => {
                    bytes += count;
                    None
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => None,
                Err(error) => Some(IoStatus::Failed(file_error(error.kind()))),
            }
        };
        if let Some(status) = status {
            return Transfer { bytes, status };
        }
    }
    Transfer {
        bytes,
        status: IoStatus::Failed(FileError::Io),
    }
}

fn write(file: &File, offset: u64, input: &[u8], progress: &AtomicU8) -> Transfer {
    let mut bytes = 0;
    for _ in 0..MAX_IO_ATTEMPTS {
        let status = if progress.load(Ordering::Acquire) == CANCELLED {
            Some(IoStatus::Cancelled)
        } else if bytes == input.len() {
            Some(IoStatus::Complete)
        } else {
            let end = input.len().min(bytes + IO_CHUNK_BYTES);
            match file.write_at(
                &input[bytes..end],
                offset + u64::try_from(bytes).expect("bounded transfer"),
            ) {
                Ok(0) => Some(IoStatus::Failed(FileError::Io)),
                Ok(count) => {
                    bytes += count;
                    None
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => None,
                Err(error) => Some(IoStatus::Failed(file_error(error.kind()))),
            }
        };
        if let Some(status) = status {
            return Transfer { bytes, status };
        }
    }
    Transfer {
        bytes,
        status: IoStatus::Failed(FileError::Io),
    }
}

fn replace(
    path: &FilePath,
    id: RequestId,
    bytes: &[u8],
    progress: &AtomicU8,
) -> io::Result<Transfer> {
    use std::io::Write;

    let path = Path::new(path.as_str());
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path.file_name().expect("validated path has a file name");
    let directory = rustix::fs::open(
        parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    match rustix::fs::statat(&directory, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(metadata) if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile => {
            return Err(io::ErrorKind::Unsupported.into());
        }
        Ok(metadata) => {
            let len = u64::try_from(metadata.st_size).map_err(|_| io::ErrorKind::InvalidData)?;
            if len > MAX_FILE_BYTES {
                return Err(io::ErrorKind::FileTooLarge.into());
            }
        }
        Err(rustix::io::Errno::NOENT) => {}
        Err(error) => return Err(error.into()),
    }
    for attempt in 0..MAX_TEMP_ATTEMPTS {
        if progress.load(Ordering::Acquire) == CANCELLED {
            return Ok(Transfer {
                bytes: 0,
                status: IoStatus::Cancelled,
            });
        }
        let mut temporary_bytes = [0_u8; 96];
        let mut temporary = io::Cursor::new(temporary_bytes.as_mut_slice());
        write!(
            temporary,
            ".gamz-{}-{}-{attempt}.tmp",
            std::process::id(),
            id.0
        )?;
        let len = usize::try_from(temporary.position()).expect("bounded temporary file name");
        let temporary =
            std::str::from_utf8(&temporary_bytes[..len]).expect("ASCII temporary file name");
        let descriptor = match rustix::fs::openat(
            &directory,
            temporary,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::from_raw_mode(0o666),
        ) {
            Ok(descriptor) => descriptor,
            Err(rustix::io::Errno::EXIST) => continue,
            Err(error) => return Err(error.into()),
        };
        let file = File::from(descriptor);
        let mut transfer = write(&file, 0, bytes, progress);
        drop(file);
        if transfer.status == IoStatus::Complete
            && progress
                .compare_exchange(ACTIVE, FINISHED, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            transfer.status = IoStatus::Cancelled;
        }
        if transfer.status == IoStatus::Complete {
            match rustix::fs::renameat(&directory, temporary, &directory, name) {
                Ok(()) => return Ok(transfer),
                Err(error) => {
                    transfer.status = IoStatus::Failed(file_error(io::Error::from(error).kind()));
                }
            }
        }
        rustix::fs::unlinkat(&directory, temporary, AtFlags::empty())?;
        transfer.bytes = 0;
        return Ok(transfer);
    }
    Err(io::ErrorKind::AlreadyExists.into())
}

fn file_error(kind: io::ErrorKind) -> FileError {
    match kind {
        io::ErrorKind::NotFound => FileError::NotFound,
        io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
        io::ErrorKind::FileTooLarge => FileError::TooLarge,
        _ => FileError::Io,
    }
}
