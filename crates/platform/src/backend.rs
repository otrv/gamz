use platform_api::services::{
    CancelStatus, PlatformEvent, ReadRequest, RequestError, RequestId, WindowChange, WriteRequest,
};

pub trait Backend {
    fn request_read(&mut self, id: RequestId, request: &ReadRequest) -> Result<(), RequestError>;
    fn request_write(
        &mut self,
        id: RequestId,
        request: &WriteRequest,
        bytes: &[u8],
    ) -> Result<(), RequestError>;
    fn request_cancel(&mut self, id: RequestId) -> CancelStatus;
    fn request_window_change(
        &mut self,
        id: RequestId,
        change: WindowChange,
    ) -> Result<(), RequestError>;
    fn request_exit(&mut self);
    fn poll(&mut self, deliver: impl FnOnce(PlatformEvent, &[u8])) -> bool;
}
