use asset_packer::OutputMode;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "Usage: asset-packer <source-directory> --max-assets <count> --max-bytes <bytes> --output <directory> [--mode embedded|external]\n\nRecursively packs regular files as opaque bytes in sorted relative-path order.\nLimits and output are required. Mode defaults to embedded.\nEmbedded mode writes only assets.gen.rs with inline pack bytes and named IDs.\nExternal mode writes only assets.pack with paths for runtime name lookup.\nSee the asset-packer README for filename rules and Cargo integration.";

fn usage_error() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, USAGE)
}

fn run() -> io::Result<()> {
    let mut arguments = env::args_os().skip(1);
    let sources = arguments.next().ok_or_else(usage_error)?;
    if sources == "--help" || sources == "-h" {
        if arguments.next().is_some() {
            return Err(usage_error());
        }
        return writeln!(io::stdout(), "{USAGE}");
    }
    let mut output = None;
    let mut max_assets = None;
    let mut max_bytes = None;
    let mut mode = None;
    for _ in 0..4 {
        let Some(option) = arguments.next() else {
            break;
        };
        let value = arguments.next().ok_or_else(usage_error)?;
        match option.to_str() {
            Some("--output") if output.is_none() => output = Some(value),
            Some("--max-assets") if max_assets.is_none() => {
                max_assets = Some(
                    value
                        .to_str()
                        .and_then(|text| text.parse().ok())
                        .ok_or_else(usage_error)?,
                );
            }
            Some("--max-bytes") if max_bytes.is_none() => {
                max_bytes = Some(
                    value
                        .to_str()
                        .and_then(|text| text.parse().ok())
                        .ok_or_else(usage_error)?,
                );
            }
            Some("--mode") if mode.is_none() => {
                mode = Some(match value.to_str() {
                    Some("embedded") => OutputMode::Embedded,
                    Some("external") => OutputMode::External,
                    _ => return Err(usage_error()),
                });
            }
            _ => return Err(usage_error()),
        }
    }
    if arguments.next().is_some() {
        return Err(usage_error());
    }
    asset_packer::build_with_mode(
        Path::new(&sources),
        Path::new(&output.ok_or_else(usage_error)?),
        max_assets.ok_or_else(usage_error)?,
        max_bytes.ok_or_else(usage_error)?,
        mode.unwrap_or(OutputMode::Embedded),
    )
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr(), "asset-packer: {error}");
            ExitCode::FAILURE
        }
    }
}
