use platform::Backend;
use platform_api::services::{
    CancelStatus, CursorMode, FileError, IoStatus, MAX_FILE_BYTES, MAX_IO_BYTES, MAX_REQUESTS,
    PlatformEvent, ReadRequest, RequestError, RequestId, WindowChange, WindowMode, WindowState,
    WindowStatus, WriteMode, WriteRequest,
};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(module = "/src/host.js")]
extern "C" {
    fn configure_services(requests: usize, io_bytes: usize, file_bytes: u32);
    #[wasm_bindgen(catch)]
    fn request_read(slot: usize, path: &str, offset: u32, length: usize) -> Result<(), JsValue>;
    #[wasm_bindgen(catch)]
    fn request_write(
        slot: usize,
        path: &str,
        offset: Option<u32>,
        bytes: &[u8],
    ) -> Result<(), JsValue>;
    fn request_cancel(slot: usize) -> bool;
    #[wasm_bindgen(catch)]
    fn request_window(
        slot: usize,
        kind: u32,
        first: u32,
        second: u32,
        title: &str,
    ) -> Result<(), JsValue>;
    fn poll_slot() -> i32;
    fn completion_status(slot: usize) -> u32;
    fn completion_count(slot: usize) -> usize;
    fn completion_payload(slot: usize) -> js_sys::Uint8Array;
    fn release_completion(slot: usize);
    fn poll_lifecycle() -> u32;
    fn has_events() -> bool;
    fn window_property(property: u32) -> u32;
}

#[derive(Clone, Copy)]
enum Pending {
    Read(RequestId),
    Write(RequestId),
    Window(RequestId),
}

impl Pending {
    fn id(self) -> RequestId {
        match self {
            Self::Read(id) | Self::Write(id) | Self::Window(id) => id,
        }
    }
}

pub(crate) struct Services {
    pending: [Option<Pending>; MAX_REQUESTS],
    exiting: bool,
}

impl Services {
    pub(crate) fn new() -> Self {
        configure_services(
            MAX_REQUESTS,
            MAX_IO_BYTES,
            u32::try_from(MAX_FILE_BYTES).expect("browser file bound fits u32"),
        );
        Self {
            pending: [None; MAX_REQUESTS],
            exiting: false,
        }
    }

    pub(crate) fn exiting(&self) -> bool {
        self.exiting
    }

    pub(crate) fn has_events(&self) -> bool {
        !self.exiting && has_events()
    }

    pub(crate) fn window_state() -> WindowState {
        WindowState {
            width: window_property(0),
            height: window_property(1),
            framebuffer_width: window_property(2),
            framebuffer_height: window_property(3),
            focused: window_property(4) != 0,
            mode: if window_property(5) == 0 {
                WindowMode::Windowed
            } else {
                WindowMode::Fullscreen
            },
            cursor: if window_property(6) == 0 {
                CursorMode::Visible
            } else {
                CursorMode::Hidden
            },
        }
    }

    fn slot(&self) -> Result<usize, RequestError> {
        if self.exiting {
            return Err(RequestError::Unavailable);
        }
        self.pending
            .iter()
            .position(Option::is_none)
            .ok_or(RequestError::QueueFull)
    }
}

impl Backend for Services {
    fn request_read(&mut self, id: RequestId, request: &ReadRequest) -> Result<(), RequestError> {
        let slot = self.slot()?;
        let offset = u32::try_from(request.offset).map_err(|_| RequestError::InvalidRange)?;
        request_read(slot, request.path.as_str(), offset, request.destination.len)
            .map_err(|_| RequestError::Unavailable)?;
        self.pending[slot] = Some(Pending::Read(id));
        Ok(())
    }

    fn request_write(
        &mut self,
        id: RequestId,
        request: &WriteRequest,
        bytes: &[u8],
    ) -> Result<(), RequestError> {
        let slot = self.slot()?;
        let offset = match request.mode {
            WriteMode::Replace => None,
            WriteMode::AtOffset(offset) => {
                Some(u32::try_from(offset).map_err(|_| RequestError::InvalidRange)?)
            }
        };
        request_write(slot, request.path.as_str(), offset, bytes)
            .map_err(|_| RequestError::Unavailable)?;
        self.pending[slot] = Some(Pending::Write(id));
        Ok(())
    }

    fn request_cancel(&mut self, id: RequestId) -> CancelStatus {
        let Some(slot) = self
            .pending
            .iter()
            .position(|pending| pending.is_some_and(|pending| pending.id() == id))
        else {
            return CancelStatus::UnknownRequest;
        };
        if request_cancel(slot) {
            CancelStatus::Requested
        } else {
            CancelStatus::TooLate
        }
    }

    fn request_window_change(
        &mut self,
        id: RequestId,
        change: WindowChange,
    ) -> Result<(), RequestError> {
        let slot = self.slot()?;
        let result = match change {
            WindowChange::Size { width, height } => request_window(slot, 0, width, height, ""),
            WindowChange::Mode(mode) => {
                request_window(slot, 1, u32::from(mode == WindowMode::Fullscreen), 0, "")
            }
            WindowChange::Cursor(mode) => {
                request_window(slot, 2, u32::from(mode == CursorMode::Hidden), 0, "")
            }
            WindowChange::Title(title) => request_window(slot, 3, 0, 0, title.as_str()),
        };
        result.map_err(|_| RequestError::Unavailable)?;
        self.pending[slot] = Some(Pending::Window(id));
        Ok(())
    }

    fn request_exit(&mut self) {
        self.exiting = true;
    }

    fn poll(&mut self, deliver: impl FnOnce(PlatformEvent, &[u8])) -> bool {
        let lifecycle = match poll_lifecycle() {
            0 => None,
            1 => Some(PlatformEvent::Suspended),
            2 => Some(PlatformEvent::Resumed),
            3 => Some(PlatformEvent::CloseRequested),
            _ => unreachable!("known browser lifecycle event"),
        };
        if let Some(event) = lifecycle {
            deliver(event, &[]);
            return true;
        }
        let Ok(slot) = usize::try_from(poll_slot()) else {
            return false;
        };
        let pending = self.pending[slot]
            .take()
            .expect("browser completed a live slot");
        let status = completion_status(slot);
        match pending {
            Pending::Read(id) => {
                let bytes = completion_payload(slot).to_vec();
                deliver(
                    PlatformEvent::ReadCompleted {
                        id,
                        bytes: bytes.len(),
                        status: io_status(status),
                    },
                    &bytes,
                );
            }
            Pending::Write(id) => deliver(
                PlatformEvent::WriteCompleted {
                    id,
                    bytes: completion_count(slot),
                    status: io_status(status),
                },
                &[],
            ),
            Pending::Window(id) => deliver(
                PlatformEvent::WindowCompleted {
                    id,
                    status: match status {
                        0 => WindowStatus::Applied,
                        2 => WindowStatus::Cancelled,
                        _ => WindowStatus::Denied,
                    },
                },
                &[],
            ),
        }
        release_completion(slot);
        true
    }
}

fn io_status(status: u32) -> IoStatus {
    match status {
        0 => IoStatus::Complete,
        1 => IoStatus::EndOfFile,
        2 => IoStatus::Cancelled,
        3 => IoStatus::Failed(FileError::NotFound),
        4 => IoStatus::Failed(FileError::PermissionDenied),
        5 => IoStatus::Failed(FileError::TooLarge),
        6 => IoStatus::Failed(FileError::Io),
        _ => unreachable!("known browser I/O status"),
    }
}
