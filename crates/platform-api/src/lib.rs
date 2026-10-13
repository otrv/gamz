#![no_std]

pub mod audio;
pub mod input;
pub mod memory;
pub mod render;
pub mod services;
pub mod timing;

pub trait Game: Sized {
    fn initialize<'p, 't>(
        persistent: memory::PersistentMemory<'p>,
        transient: memory::TransientMemory<'t>,
        api: &services::PlatformApi,
    ) -> Result<(&'p mut Self, impl AsRef<[render::TextureUpload<'t>]>), services::StartupError>;

    fn update<'t>(
        &mut self,
        memory: memory::TransientMemory<'t>,
        input: &input::FrameInput,
    ) -> render::Frame<'t>;

    fn update_audio(&mut self, buffer: audio::AudioBuffer<'_>);
}
