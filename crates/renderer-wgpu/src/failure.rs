use crate::{RendererError, error};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(super) struct Failure(Arc<Mutex<Option<String>>>);

impl Failure {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let failure = Arc::new(Mutex::new(None));
        let errors = Arc::clone(&failure);
        device.on_uncaptured_error(Arc::new(move |value| {
            assert!(!matches!(value, wgpu::Error::Validation { .. }), "{value}");
            let mut stored = errors.lock().unwrap();
            if stored.is_none() {
                *stored = Some(value.to_string());
            }
        }));
        let lost = Arc::clone(&failure);
        device.set_device_lost_callback(move |reason, message| {
            let mut stored = lost.lock().unwrap();
            if stored.is_none() {
                *stored = Some(format!("device lost ({reason:?}): {message}"));
            }
        });
        Self(failure)
    }

    pub(super) fn check(&self) -> Result<(), RendererError> {
        self.0
            .lock()
            .unwrap()
            .as_ref()
            .map_or(Ok(()), |message| Err(error(message)))
    }

    pub(super) fn wait(
        &self,
        device: &wgpu::Device,
        submission_index: Option<wgpu::SubmissionIndex>,
    ) -> Result<(), RendererError> {
        self.check()?;
        let _ = device
            .poll(wgpu::PollType::Wait {
                submission_index,
                timeout: Some(Duration::from_secs(5)),
            })
            .map_err(error)?;
        self.check()
    }
}
