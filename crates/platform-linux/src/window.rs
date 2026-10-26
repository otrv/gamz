use std::time::{Duration, Instant};

use platform_api::input::PlatformEvent;
use platform_api::request::{CancelStatus, RequestError, RequestId};
use platform_api::window::{WindowMode, WindowStatus};

use crate::limits::MAX_REQUESTS;

const CHANGE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
enum Progress {
    Queued,
    Applying(Instant),
    Finished(WindowStatus),
}

#[derive(Clone, Copy)]
struct Request {
    id: RequestId,
    mode: WindowMode,
    progress: Progress,
}

pub(crate) struct ModeRequests {
    requests: [Option<Request>; MAX_REQUESTS],
    next_id: u64,
}

impl ModeRequests {
    pub(crate) fn new() -> Self {
        Self {
            requests: [None; MAX_REQUESTS],
            next_id: 2,
        }
    }

    pub(crate) fn request(&mut self, mode: WindowMode) -> Result<RequestId, RequestError> {
        let slot = self
            .requests
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(RequestError::QueueFull)?;
        let id = RequestId(self.next_id);
        self.next_id = self.next_id.checked_add(2).ok_or(RequestError::QueueFull)?;
        *slot = Some(Request {
            id,
            mode,
            progress: Progress::Queued,
        });
        Ok(id)
    }

    pub(crate) fn cancel(&mut self, id: RequestId) -> CancelStatus {
        let Some(request) = self
            .requests
            .iter_mut()
            .flatten()
            .find(|request| request.id == id)
        else {
            return CancelStatus::UnknownRequest;
        };
        match request.progress {
            Progress::Queued | Progress::Finished(WindowStatus::Cancelled) => {
                request.progress = Progress::Finished(WindowStatus::Cancelled);
                CancelStatus::Requested
            }
            Progress::Applying(_) | Progress::Finished(_) => CancelStatus::TooLate,
        }
    }

    pub(crate) fn apply(&mut self, current: WindowMode, mut set: impl FnMut(WindowMode)) {
        let now = Instant::now();
        for request in self.requests.iter_mut().flatten() {
            if let Progress::Applying(deadline) = request.progress {
                if current == request.mode {
                    request.progress = Progress::Finished(WindowStatus::Applied);
                } else if now >= deadline {
                    request.progress = Progress::Finished(WindowStatus::Denied);
                } else {
                    return;
                }
            }
        }
        let next = self
            .requests
            .iter_mut()
            .flatten()
            .filter(|request| matches!(request.progress, Progress::Queued))
            .min_by_key(|request| request.id.0);
        if let Some(request) = next {
            if current == request.mode {
                request.progress = Progress::Finished(WindowStatus::Applied);
            } else {
                set(request.mode);
                request.progress = Progress::Applying(now + CHANGE_TIMEOUT);
            }
        }
    }

    pub(crate) fn events(&mut self, output: &mut [PlatformEvent<'_>]) -> usize {
        let mut count = 0;
        for slot in &mut self.requests {
            if count == output.len() {
                break;
            }
            if let Some(Request {
                id,
                progress: Progress::Finished(status),
                ..
            }) = *slot
            {
                output[count] = PlatformEvent::WindowModeCompleted { id, status };
                *slot = None;
                count += 1;
            }
        }
        count
    }
}
