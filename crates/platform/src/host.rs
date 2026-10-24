use platform_api::Platform;
use platform_api::memory::MemoryRange;
use platform_api::services::{
    CancelStatus, MAX_EVENTS, MAX_FILE_BYTES, MAX_IO_BYTES, MAX_REQUESTS, PlatformEvent,
    ReadRequest, RequestError, RequestId, WindowChange, WriteMode, WriteRequest,
};

use crate::Backend;

#[derive(Clone, Copy)]
enum Pending {
    Empty,
    Read { id: RequestId, range: MemoryRange },
    Write { id: RequestId, range: MemoryRange },
    Window { id: RequestId },
}

impl Pending {
    fn id(self) -> Option<RequestId> {
        match self {
            Self::Empty => None,
            Self::Read { id, .. } | Self::Write { id, .. } | Self::Window { id } => Some(id),
        }
    }
}

pub struct Host<'p, B: Backend> {
    backend: &'p mut B,
    persistent: &'p mut [u8],
    pending: [Pending; MAX_REQUESTS],
    next_request: u64,
}

impl<'p, B: Backend> Host<'p, B> {
    pub fn new(backend: &'p mut B) -> Self {
        Self {
            backend,
            persistent: &mut [],
            pending: [Pending::Empty; MAX_REQUESTS],
            next_request: 1,
        }
    }

    #[must_use]
    pub fn backend(&self) -> &B {
        self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        self.backend
    }

    pub fn poll_events(&mut self, events: &mut [PlatformEvent]) -> usize {
        assert!(events.len() <= MAX_EVENTS);
        let mut count = 0;
        for slot in events {
            if !self.backend.poll(|event, bytes| {
                complete(self.persistent, &mut self.pending, event, bytes);
                *slot = event;
            }) {
                break;
            }
            count += 1;
        }
        count
    }
}

impl<'p, B: Backend> Platform<'p> for Host<'p, B> {
    fn set_persistent_memory(&mut self, bytes: &'p mut [u8]) {
        assert!(self.persistent.is_empty(), "I/O memory is bound only once");
        assert!(self.pending.iter().all(|pending| pending.id().is_none()));
        self.persistent = bytes;
    }

    fn persistent_capacity(&self) -> usize {
        self.persistent.len()
    }

    fn memory(&self, range: MemoryRange) -> Result<&[u8], RequestError> {
        self.check_range(range)?;
        for pending in self.pending {
            if let Pending::Read {
                range: reserved, ..
            } = pending
                && overlaps(range, reserved)
            {
                return Err(RequestError::MemoryBusy);
            }
        }
        Ok(&self.persistent[range.offset..range.offset + range.len])
    }

    fn memory_mut(&mut self, range: MemoryRange) -> Result<&mut [u8], RequestError> {
        self.check_available(range)?;
        Ok(&mut self.persistent[range.offset..range.offset + range.len])
    }

    fn request_read(&mut self, request: &ReadRequest) -> Result<RequestId, RequestError> {
        self.check_io_range(request.destination)?;
        request
            .offset
            .checked_add(
                u64::try_from(request.destination.len).map_err(|_| RequestError::InvalidRange)?,
            )
            .filter(|&end| end <= MAX_FILE_BYTES)
            .ok_or(RequestError::InvalidRange)?;
        let (slot, id) = self.next_slot()?;
        self.backend.request_read(id, request)?;
        self.pending[slot] = Pending::Read {
            id,
            range: request.destination,
        };
        Ok(id)
    }

    fn request_write(&mut self, request: &WriteRequest) -> Result<RequestId, RequestError> {
        self.check_io_range(request.source)?;
        if let WriteMode::AtOffset(offset) = request.mode {
            offset
                .checked_add(
                    u64::try_from(request.source.len).map_err(|_| RequestError::InvalidRange)?,
                )
                .filter(|&end| end <= MAX_FILE_BYTES)
                .ok_or(RequestError::InvalidRange)?;
        }
        let (slot, id) = self.next_slot()?;
        let range = request.source;
        let bytes = &self.persistent[range.offset..range.offset + range.len];
        self.backend.request_write(id, request, bytes)?;
        self.pending[slot] = Pending::Write {
            id,
            range: request.source,
        };
        Ok(id)
    }

    fn request_cancel(&mut self, id: RequestId) -> CancelStatus {
        if !self.pending.iter().any(|pending| pending.id() == Some(id)) {
            return CancelStatus::UnknownRequest;
        }
        self.backend.request_cancel(id)
    }

    fn request_window_change(&mut self, change: WindowChange) -> Result<RequestId, RequestError> {
        if let WindowChange::Size { width, height } = change
            && (!(1..=16_384).contains(&width) || !(1..=16_384).contains(&height))
        {
            return Err(RequestError::InvalidValue);
        }
        let (slot, id) = self.next_slot()?;
        self.backend.request_window_change(id, change)?;
        self.pending[slot] = Pending::Window { id };
        Ok(id)
    }

    fn request_exit(&mut self) {
        self.backend.request_exit();
    }
}

impl<B: Backend> Host<'_, B> {
    fn check_range(&self, range: MemoryRange) -> Result<(), RequestError> {
        if range
            .offset
            .checked_add(range.len)
            .is_none_or(|end| end > self.persistent.len())
        {
            return Err(RequestError::InvalidRange);
        }
        Ok(())
    }

    fn check_available(&self, range: MemoryRange) -> Result<(), RequestError> {
        self.check_range(range)?;
        for pending in self.pending {
            match pending {
                Pending::Read {
                    range: reserved, ..
                }
                | Pending::Write {
                    range: reserved, ..
                } if overlaps(range, reserved) => return Err(RequestError::MemoryBusy),
                _ => {}
            }
        }
        Ok(())
    }

    fn check_io_range(&self, range: MemoryRange) -> Result<(), RequestError> {
        if range.len > MAX_IO_BYTES {
            return Err(RequestError::InvalidRange);
        }
        self.check_available(range)
    }

    fn next_slot(&mut self) -> Result<(usize, RequestId), RequestError> {
        let slot = self
            .pending
            .iter()
            .position(|pending| matches!(pending, Pending::Empty))
            .ok_or(RequestError::QueueFull)?;
        let id = RequestId::new(self.next_request);
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or(RequestError::QueueFull)?;
        Ok((slot, id))
    }
}

fn overlaps(range: MemoryRange, other: MemoryRange) -> bool {
    range.len != 0
        && other.len != 0
        && range.offset
            < other
                .offset
                .checked_add(other.len)
                .expect("validated range")
        && other.offset
            < range
                .offset
                .checked_add(range.len)
                .expect("validated range")
}

fn complete(
    persistent: &mut [u8],
    pending: &mut [Pending; MAX_REQUESTS],
    event: PlatformEvent,
    bytes: &[u8],
) {
    let id = match event {
        PlatformEvent::ReadCompleted { id, .. }
        | PlatformEvent::WriteCompleted { id, .. }
        | PlatformEvent::WindowCompleted { id, .. } => id,
        PlatformEvent::CloseRequested | PlatformEvent::Suspended | PlatformEvent::Resumed => {
            assert_eq!(bytes, []);
            return;
        }
    };
    let slot = pending
        .iter()
        .position(|pending| pending.id() == Some(id))
        .expect("host completed a pending request exactly once");
    match (pending[slot], event) {
        (Pending::Read { range, .. }, PlatformEvent::ReadCompleted { bytes: count, .. }) => {
            assert_eq!(bytes.len(), count);
            assert!(count <= range.len);
            persistent[range.offset..range.offset + count].copy_from_slice(bytes);
        }
        (Pending::Write { range, .. }, PlatformEvent::WriteCompleted { bytes: count, .. }) => {
            assert_eq!(bytes, []);
            assert!(count <= range.len);
        }
        (Pending::Window { .. }, PlatformEvent::WindowCompleted { .. }) => assert_eq!(bytes, []),
        _ => panic!("host completion does not match the request kind"),
    }
    pending[slot] = Pending::Empty;
}
