#[derive(Clone, Copy)]
pub struct ReadRequest<'a> {
    pub path: &'a str,
    pub offset: u64,
    pub bytes: usize,
}

#[derive(Clone, Copy)]
pub struct WriteRequest<'a> {
    pub path: &'a str,
    pub bytes: &'a [u8],
    pub mode: WriteMode,
}

#[derive(Clone, Copy)]
pub enum WriteMode {
    Replace,
    AtOffset(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileError {
    NotFound,
    PermissionDenied,
    TooLarge,
    Io,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IoStatus {
    Complete,
    EndOfFile,
    Cancelled,
    Failed(FileError),
}
