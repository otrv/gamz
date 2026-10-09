#[cfg(not(target_os = "linux"))]
compile_error!("the CLI platform is implemented only for Linux");

mod input;
#[path = "../memory.rs"]
mod memory;
mod output;
#[path = "../linux/services.rs"]
mod services;

use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::process::ExitCode;

use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::services::PlatformApi;
use serde::Serialize;

use input::Input;
use memory::{PERSISTENT_MEMORY_BYTES, TRANSIENT_MEMORY_BYTES};
use output::{FrameOutput, Startup, Uploads};
use services::load_entire_file;

const MAX_INPUT_BYTES: usize = 16 * 1024;
const MAX_MEMORY_BYTES: usize = 1024 * 1024 * 1024;

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
    let mut persistent = Vec::<u8>::new();
    persistent.try_reserve_exact(persistent_bytes)?;
    let mut transient = Vec::<u8>::new();
    transient.try_reserve_exact(transient_bytes)?;
    let mut output = BufWriter::new(io::stdout().lock());
    let (state, uploads) = game::initialize(
        PersistentMemory {
            bytes: &mut persistent.spare_capacity_mut()[..persistent_bytes],
        },
        TransientMemory {
            bytes: &mut transient.spare_capacity_mut()[..transient_bytes],
        },
        &PlatformApi { load_entire_file },
    )
    .map_err(|error| io::Error::other(error.to_string()))?;
    write_line(
        &mut output,
        &Startup {
            uploads: Uploads(&uploads),
        },
    )?;

    let mut input = BufReader::new(io::stdin().lock());
    let mut line = Vec::with_capacity(MAX_INPUT_BYTES + 1);
    loop {
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
        let Input(input) = serde_json::from_slice(&line)?;
        let frame = game::update(
            state,
            TransientMemory {
                bytes: &mut transient.spare_capacity_mut()[..transient_bytes],
            },
            &input,
        );
        write_line(&mut output, &FrameOutput(&frame))?;
    }
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
