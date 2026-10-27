#[cfg(not(target_os = "linux"))]
compile_error!("the CLI platform is implemented only for Linux");

#[path = "../controls.rs"]
mod controls;
#[path = "../files.rs"]
mod files;
mod frame;
mod input;
#[path = "../limits.rs"]
mod limits;
#[path = "../memory.rs"]
mod memory;
mod output;
#[path = "../textures.rs"]
mod textures;
#[path = "../window.rs"]
mod window;

use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::mem::MaybeUninit;
use std::process::ExitCode;

use platform_api::audio::StereoSample;
use platform_api::input::{self as contract, PlatformEvent};
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::window::{WindowMode, WindowState};
use platform_api::{Game, Platform};
use serde::Serialize;

use files::{Completions, Files};
use input::Input;
use limits::{MAX_AUDIO_FRAMES, MAX_EVENTS};
use memory::{COMMAND_MEMORY_BYTES, PERSISTENT_MEMORY_BYTES, TRANSIENT_MEMORY_BYTES};
use output::{AudioOutput, FrameOutput, Startup, Uploads};
use textures::Textures;
use window::ModeRequests;

const MAX_INPUT_BYTES: usize = 16 * 1024;
const MAX_MEMORY_BYTES: usize = 1024 * 1024 * 1024;

struct Context {
    files: Files,
    textures: Textures,
    modes: ModeRequests,
    exit: bool,
}

impl Context {
    fn api(&mut self) -> Platform<'_, Self> {
        Platform {
            context: self,
            read_file: |context, request| context.files.read(request),
            write_file: |context, request| context.files.write(request),
            cancel: |context, id| {
                if id.0 % 2 == 0 {
                    context.modes.cancel(id)
                } else {
                    context.files.cancel(id)
                }
            },
            upload_texture: |context, upload| context.textures.upload(upload),
            release_texture: |context, id| context.textures.release(id),
            set_window_mode: |context, mode| context.modes.request(mode),
            exit: |context| context.exit = true,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gamz cli: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut persistent_bytes = PERSISTENT_MEMORY_BYTES;
    let mut transient_bytes = TRANSIENT_MEMORY_BYTES;
    let mut args = std::env::args_os().skip(1);
    for _ in 0..2 {
        let Some(flag) = args.next() else { break };
        let destination = match flag.to_str() {
            Some("--persistent-bytes") => &mut persistent_bytes,
            Some("--transient-bytes") => &mut transient_bytes,
            _ => return Err("expected --persistent-bytes or --transient-bytes".into()),
        };
        *destination = args
            .next()
            .and_then(|value| value.to_str().and_then(|value| value.parse().ok()))
            .filter(|value| *value <= MAX_MEMORY_BYTES)
            .ok_or("memory size must be an integer between 0 and 1073741824")?;
    }
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
    let mut persistent = memory::allocate(persistent_bytes)?;
    let mut transient = memory::allocate(transient_bytes)?;
    let mut output = BufWriter::new(io::stdout().lock());
    let mut context = Context {
        files: Files::new(),
        textures: Textures::new(),
        modes: ModeRequests::new(),
        exit: false,
    };
    let state = game::GameState::initialize(
        PersistentMemory {
            bytes: &mut persistent.spare_capacity_mut()[..persistent_bytes],
        },
        transient_bytes,
        &mut context.api(),
    )
    .map_err(|error| format!("game initialization: {error:?}"))?;
    write_line(
        &mut output,
        &Startup {
            uploads: Uploads(&context.textures.uploads),
        },
    )?;
    context.textures.clear_pending();
    drive(
        state,
        &mut context,
        &mut transient.spare_capacity_mut()[..transient_bytes],
        &mut output,
    )
}

fn drive(
    state: &mut game::GameState,
    context: &mut Context,
    transient: &mut [MaybeUninit<u8>],
    output: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command_bytes = memory::allocate(COMMAND_MEMORY_BYTES)?;
    let mut audio = vec![StereoSample { left: 0, right: 0 }; MAX_AUDIO_FRAMES];
    let mut completions = Completions::new();
    let mut window = WindowState {
        width: 1280,
        height: 720,
        framebuffer_width: 1280,
        framebuffer_height: 720,
        focused: true,
        mode: WindowMode::Windowed,
    };
    let mut input = BufReader::new(io::stdin().lock());
    let mut line = Vec::with_capacity(MAX_INPUT_BYTES + 1);
    while !context.exit {
        line.clear();
        let count = input
            .by_ref()
            .take(u64::try_from(MAX_INPUT_BYTES + 1).unwrap())
            .read_until(b'\n', &mut line)?;
        if count == 0 {
            return Ok(());
        }
        if count > MAX_INPUT_BYTES {
            return Err("input line exceeds 16384 bytes".into());
        }
        match serde_json::from_slice(&line)? {
            Input::Frame {
                dt,
                controller,
                close_requested,
                pointer,
                scroll,
                text,
            } => {
                context.modes.apply(window.mode, |mode| window.mode = mode);
                context.files.collect(&mut completions);
                let mut events = [PlatformEvent::Resumed; MAX_EVENTS];
                let mut count = completions.events(&mut events);
                count += context.modes.events(&mut events[count..]);
                if close_requested {
                    events[count] = PlatformEvent::CloseRequested;
                    count += 1;
                }
                let input = contract::Input {
                    elapsed: dt,
                    actions: controller,
                    pointer,
                    scroll,
                    text: &text,
                    window,
                    events: &events[..count],
                };
                let frame = state
                    .update(
                        &input,
                        TransientMemory { bytes: transient },
                        memory::commands(
                            &mut command_bytes.spare_capacity_mut()[..COMMAND_MEMORY_BYTES],
                        ),
                        &mut context.api(),
                    )
                    .map_err(|error| format!("game frame: {error:?}"))?;
                frame::validate(&frame)?;
                write_line(
                    output,
                    &FrameOutput {
                        frame: &frame,
                        uploads: Uploads(&context.textures.uploads),
                        releases: &context.textures.releases,
                    },
                )?;
                context.textures.clear_pending();
                context.files.recycle(&mut completions);
            }
            Input::Audio {
                sample_rate,
                frames,
            } => {
                let frames = &mut audio[..frames];
                state.update_audio(sample_rate, frames);
                write_line(
                    output,
                    &AudioOutput {
                        sample_rate,
                        frames,
                    },
                )?;
            }
        }
    }
    Ok(())
}

fn write_line(
    output: &mut impl Write,
    value: &impl Serialize,
) -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
