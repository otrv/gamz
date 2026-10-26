#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestError {
    QueueFull,
    InvalidPath,
    InvalidRange,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelStatus {
    Requested,
    TooLate,
    UnknownRequest,
}
