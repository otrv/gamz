use crate::file::{ReadRequest, WriteRequest};
use crate::render::{TextureError, TextureId, TextureUpload};
use crate::request::{CancelStatus, RequestError, RequestId};
use crate::window::WindowMode;

pub struct Platform<'a, Context> {
    pub context: &'a mut Context,
    pub read_file: fn(&mut Context, ReadRequest<'_>) -> Result<RequestId, RequestError>,
    pub write_file: fn(&mut Context, WriteRequest<'_>) -> Result<RequestId, RequestError>,
    pub cancel: fn(&mut Context, RequestId) -> CancelStatus,
    pub upload_texture: fn(&mut Context, TextureUpload<'_>) -> Result<(), TextureError>,
    pub release_texture: fn(&mut Context, TextureId) -> Result<(), TextureError>,
    pub set_window_mode: fn(&mut Context, WindowMode) -> Result<RequestId, RequestError>,
    pub exit: fn(&mut Context),
}
