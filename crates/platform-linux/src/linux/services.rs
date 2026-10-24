use std::array;
use std::collections::VecDeque;
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, TryLockError};
use std::thread::{self, JoinHandle};

use platform::Backend;
use platform_api::services::{
    CancelStatus, FileError, FilePath, IoStatus, MAX_FILE_BYTES, MAX_IO_BYTES, MAX_REQUESTS,
    PlatformEvent, ReadRequest, RequestError, RequestId, WindowChange, WindowState, WindowStatus,
    WriteMode, WriteRequest,
};
use rustix::fs::{AtFlags, FileType, Mode, OFlags};

const ACTIVE: u8 = 0;
const CANCELLED: u8 = 1;
const FINISHED: u8 = 2;
const MAX_IO_ATTEMPTS: usize = 4096;
const IO_CHUNK_BYTES: usize = 64 * 1024;
const MAX_TEMP_ATTEMPTS: usize = 16;

#[derive(Clone, Copy)]
enum IoKind {
    Read,
    Write,
}

enum WindowProgress {
    Queued(WindowChange),
    Applying,
    Finished(WindowStatus),
}

enum Slot {
    Idle,
    Io {
        id: RequestId,
        kind: IoKind,
    },
    Window {
        id: RequestId,
        progress: WindowProgress,
    },
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

pub(crate) struct Services {
    worker: Arc<Worker>,
    thread: Option<JoinHandle<()>>,
    buffers: [Option<Vec<u8>>; MAX_REQUESTS],
    slots: [Slot; MAX_REQUESTS],
    window: WindowState,
    close_requested: bool,
    exit_requested: bool,
}

impl Services {
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
            slots: array::from_fn(|_| Slot::Idle),
            window: WindowState::default(),
            close_requested: false,
            exit_requested: false,
        }
    }

    pub(crate) fn take_window_request(&mut self) -> Option<(RequestId, WindowChange)> {
        for slot in &mut self.slots {
            if let Slot::Window {
                id,
                progress: WindowProgress::Queued(change),
            } = slot
            {
                let request = (*id, *change);
                *slot = Slot::Window {
                    id: *id,
                    progress: WindowProgress::Applying,
                };
                return Some(request);
            }
        }
        None
    }

    pub(crate) fn complete_window(&mut self, id: RequestId, status: WindowStatus) {
        for slot in &mut self.slots {
            if let Slot::Window {
                id: pending,
                progress,
            } = slot
                && *pending == id
                && matches!(progress, WindowProgress::Applying)
            {
                *progress = WindowProgress::Finished(status);
                return;
            }
        }
    }

    pub(crate) fn set_window_state(&mut self, state: WindowState) {
        self.window = state;
    }

    pub(crate) fn window_state(&self) -> WindowState {
        self.window
    }

    pub(crate) fn request_close(&mut self) {
        self.close_requested = true;
    }

    pub(crate) fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    fn available_slot(&self, id: RequestId) -> Result<usize, RequestError> {
        if self.slots.iter().any(|slot| match slot {
            Slot::Io { id: pending, .. } | Slot::Window { id: pending, .. } => *pending == id,
            Slot::Idle => false,
        }) {
            return Err(RequestError::InvalidValue);
        }
        self.slots
            .iter()
            .position(|slot| matches!(slot, Slot::Idle))
            .ok_or(RequestError::QueueFull)
    }

    fn submit(
        &mut self,
        id: RequestId,
        operation: Operation,
        bytes: &[u8],
    ) -> Result<(), RequestError> {
        let slot = self.available_slot(id)?;
        let kind = match operation {
            Operation::Read { .. } => IoKind::Read,
            Operation::Write { .. } => IoKind::Write,
        };
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
        buffer[..bytes.len()].copy_from_slice(bytes);
        self.worker.progress[slot].store(ACTIVE, Ordering::Release);
        self.slots[slot] = Slot::Io { id, kind };
        queue.pending.push_back(Job {
            slot,
            id,
            operation,
            buffer,
        });
        drop(queue);
        self.worker.wake.notify_one();
        Ok(())
    }
}

impl Backend for Services {
    fn request_read(&mut self, id: RequestId, request: &ReadRequest) -> Result<(), RequestError> {
        validate_io_range(request.offset, request.destination.len)?;
        self.submit(
            id,
            Operation::Read {
                path: request.path,
                offset: request.offset,
                len: request.destination.len,
            },
            &[],
        )
    }

    fn request_write(
        &mut self,
        id: RequestId,
        request: &WriteRequest,
        bytes: &[u8],
    ) -> Result<(), RequestError> {
        if bytes.len() != request.source.len {
            return Err(RequestError::InvalidRange);
        }
        let offset = match request.mode {
            WriteMode::Replace => 0,
            WriteMode::AtOffset(offset) => offset,
        };
        validate_io_range(offset, bytes.len())?;
        self.submit(
            id,
            Operation::Write {
                path: request.path,
                mode: request.mode,
                len: bytes.len(),
            },
            bytes,
        )
    }

    fn request_cancel(&mut self, id: RequestId) -> CancelStatus {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            match slot {
                Slot::Io { id: pending, .. } if *pending == id => {
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
                Slot::Window {
                    id: pending,
                    progress,
                } if *pending == id => {
                    return match progress {
                        WindowProgress::Queued(_) => {
                            *progress = WindowProgress::Finished(WindowStatus::Cancelled);
                            CancelStatus::Requested
                        }
                        WindowProgress::Applying | WindowProgress::Finished(_) => {
                            CancelStatus::TooLate
                        }
                    };
                }
                _ => {}
            }
        }
        CancelStatus::UnknownRequest
    }

    fn request_window_change(
        &mut self,
        id: RequestId,
        change: WindowChange,
    ) -> Result<(), RequestError> {
        if let WindowChange::Size { width, height } = change
            && (width == 0 || height == 0)
        {
            return Err(RequestError::InvalidValue);
        }
        let slot = self.available_slot(id)?;
        self.slots[slot] = Slot::Window {
            id,
            progress: WindowProgress::Queued(change),
        };
        Ok(())
    }

    fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    fn poll(&mut self, deliver: impl FnOnce(PlatformEvent, &[u8])) -> bool {
        let completion = match self.worker.queue.try_lock() {
            Ok(mut queue) => queue.completed.pop_front(),
            Err(TryLockError::WouldBlock) => None,
            Err(TryLockError::Poisoned(_)) => panic!("file worker queue poisoned"),
        };
        if let Some(Completion { job, transfer }) = completion {
            let Slot::Io { id, kind } = self.slots[job.slot] else {
                panic!("completion must retain its reserved request slot");
            };
            assert_eq!(id, job.id);
            let event = match kind {
                IoKind::Read => PlatformEvent::ReadCompleted {
                    id,
                    bytes: transfer.bytes,
                    status: transfer.status,
                },
                IoKind::Write => PlatformEvent::WriteCompleted {
                    id,
                    bytes: transfer.bytes,
                    status: transfer.status,
                },
            };
            let data = match kind {
                IoKind::Read => &job.buffer[..transfer.bytes],
                IoKind::Write => &[],
            };
            deliver(event, data);
            self.buffers[job.slot] = Some(job.buffer);
            self.slots[job.slot] = Slot::Idle;
            return true;
        }
        for slot in &mut self.slots {
            if let Slot::Window {
                id,
                progress: WindowProgress::Finished(status),
            } = slot
            {
                let event = PlatformEvent::WindowCompleted {
                    id: *id,
                    status: *status,
                };
                *slot = Slot::Idle;
                deliver(event, &[]);
                return true;
            }
        }
        if self.close_requested {
            self.close_requested = false;
            deliver(PlatformEvent::CloseRequested, &[]);
            return true;
        }
        false
    }
}

impl Drop for Services {
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

fn validate_io_range(offset: u64, len: usize) -> Result<(), RequestError> {
    let len_u64 = u64::try_from(len).map_err(|_| RequestError::InvalidRange)?;
    if len > MAX_IO_BYTES
        || offset
            .checked_add(len_u64)
            .is_none_or(|end| end > MAX_FILE_BYTES)
    {
        return Err(RequestError::InvalidRange);
    }
    Ok(())
}

fn perform(job: &mut Job, progress: &AtomicU8) -> io::Result<Transfer> {
    match job.operation {
        Operation::Read { path, offset, len } => {
            let file = open_regular(path.as_str(), OFlags::RDONLY)?;
            Ok(read(&file, offset, &mut job.buffer[..len], progress))
        }
        Operation::Write {
            path,
            mode: WriteMode::AtOffset(offset),
            len,
        } => {
            let file = open_regular(path.as_str(), OFlags::WRONLY | OFlags::CREATE)?;
            Ok(write(&file, offset, &job.buffer[..len], progress))
        }
        Operation::Write {
            path,
            mode: WriteMode::Replace,
            len,
        } => replace(&path, job.id, &job.buffer[..len], progress),
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
            id.get()
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
