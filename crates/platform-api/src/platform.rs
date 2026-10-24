use crate::memory::MemoryRange;
use crate::services::{
    CancelStatus, ReadRequest, RequestError, RequestId, WindowChange, WriteRequest,
};

pub trait Platform<'p> {
    fn set_persistent_memory(&mut self, bytes: &'p mut [u8]);
    fn persistent_capacity(&self) -> usize;
    fn memory(&self, range: MemoryRange) -> Result<&[u8], RequestError>;
    fn memory_mut(&mut self, range: MemoryRange) -> Result<&mut [u8], RequestError>;
    fn request_read(&mut self, request: &ReadRequest) -> Result<RequestId, RequestError>;
    fn request_write(&mut self, request: &WriteRequest) -> Result<RequestId, RequestError>;
    fn request_cancel(&mut self, id: RequestId) -> CancelStatus;
    fn request_window_change(&mut self, change: WindowChange) -> Result<RequestId, RequestError>;
    fn request_exit(&mut self);
}
