use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::{env, fs};

use serde_json::Value;

const WEB_TARGET: &str = "wasm32-unknown-unknown";
const TARGETS: &[&str] = &["x86_64-unknown-linux-gnu", WEB_TARGET];
const UNSAFE_CRATES: &[&str] = &["game-memory"];
const MIRI_TOOLCHAIN: &str = "nightly-2026-10-01";
const DENY_VERSION: &str = "0.20.2";
const DENY_ARCHIVE_SHA256: &str =
    "9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f";
const WASM_BINDGEN_VERSION: &str = "0.2.129";
const MAX_TOOL_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;

fn cargo(arguments: &[&str]) -> Command {
    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command.args(arguments);
    command
}

fn miri(arguments: &[&str]) -> Command {
    let mut command = Command::new("rustup");
    command.args(["run", MIRI_TOOLCHAIN, "cargo", "miri"]);
    command.args(arguments);
    command
}

fn run(command: &mut Command) -> io::Result<()> {
    println!("==> {command:?}");
    let status = command.status()?;
    if !status.success() {
        return Err(io::Error::other(format!("{command:?} failed: {status}")));
    }
    Ok(())
}

fn capture(command: &mut Command) -> io::Result<String> {
    let mut child = command.stdout(Stdio::piped()).spawn()?;
    let mut bytes = Vec::new();
    let result = child
        .stdout
        .take()
        .expect("stdout was piped")
        .take(MAX_TOOL_OUTPUT_BYTES + 1)
        .read_to_end(&mut bytes);
    if result.is_err() || bytes.len() > usize::try_from(MAX_TOOL_OUTPUT_BYTES).unwrap() {
        let _ = child.kill();
        let _ = child.wait();
        result?;
        return Err(io::Error::other(format!(
            "{command:?} exceeded output budget"
        )));
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(io::Error::other(format!("{command:?} failed: {status}")));
    }
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn deny_installed() -> bool {
    capture(cargo(&["deny", "--version"]).stderr(Stdio::null()))
        .is_ok_and(|version| version.trim_end_matches('\n') == format!("cargo-deny {DENY_VERSION}"))
}

fn wasm_bindgen_installed() -> bool {
    capture(
        Command::new("wasm-bindgen")
            .arg("--version")
            .stderr(Stdio::null()),
    )
    .is_ok_and(|version| version.trim() == format!("wasm-bindgen {WASM_BINDGEN_VERSION}"))
}

fn miri_installed() -> bool {
    capture(miri(&["--version"]).stderr(Stdio::null())).is_ok()
        && capture(
            Command::new("rustup")
                .args([
                    "component",
                    "list",
                    "--toolchain",
                    MIRI_TOOLCHAIN,
                    "--installed",
                ])
                .stderr(Stdio::null()),
        )
        .is_ok_and(|components| components.lines().any(|component| component == "rust-src"))
}

fn setup() -> io::Result<()> {
    for target in TARGETS {
        run(Command::new("rustup").args(["target", "add", target]))?;
    }
    if !deny_installed() {
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            run(Command::new("sh").args([
                "-eu",
                "-c",
                r#"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
release="cargo-deny-$1-x86_64-unknown-linux-musl"
archive="$temporary/$release.tar.gz"
url="https://github.com/EmbarkStudios/cargo-deny/releases/download/$1/$release.tar.gz"
curl --proto '=https' --tlsv1.2 -fsSL "$url" -o "$archive"
printf '%s  %s\n' "$2" "$archive" | sha256sum --check --strict -
tar -xzf "$archive" -C "$temporary" "$release/cargo-deny"
test "$("$temporary/$release/cargo-deny" --version)" = "cargo-deny $1"
install -Dm755 "$temporary/$release/cargo-deny" "${CARGO_HOME:-"$HOME/.cargo"}/bin/cargo-deny"
"#,
                "install-cargo-deny",
                DENY_VERSION,
                DENY_ARCHIVE_SHA256,
            ]))?;
        } else {
            run(&mut cargo(&[
                "install",
                "cargo-deny",
                "--version",
                &format!("={DENY_VERSION}"),
                "--locked",
            ]))?;
        }
    }
    if !wasm_bindgen_installed() {
        run(&mut cargo(&[
            "install",
            "wasm-bindgen-cli",
            "--version",
            &format!("={WASM_BINDGEN_VERSION}"),
            "--locked",
        ]))?;
    }
    if !miri_installed() {
        run(Command::new("rustup").args([
            "toolchain",
            "install",
            MIRI_TOOLCHAIN,
            "--profile",
            "minimal",
            "--component",
            "miri,rust-src",
            "--no-self-update",
        ]))?;
    }
    prerequisites()
}

fn prerequisites() -> io::Result<()> {
    if !deny_installed() || !miri_installed() || !wasm_bindgen_installed() {
        return Err(io::Error::other(format!(
            "checks require cargo-deny {DENY_VERSION}, wasm-bindgen {WASM_BINDGEN_VERSION}, and Miri with rust-src on {MIRI_TOOLCHAIN}; run cargo xtask setup"
        )));
    }
    let installed = capture(Command::new("rustup").args(["target", "list", "--installed"]))?;
    for target in TARGETS {
        if !installed.lines().any(|line| line == *target) {
            return Err(io::Error::other(format!(
                "target {target} is missing; run cargo xtask setup"
            )));
        }
    }
    Ok(())
}

fn game_selections() -> io::Result<Vec<String>> {
    let metadata: Value = serde_json::from_str(&capture(&mut cargo(&[
        "metadata",
        "--locked",
        "--format-version",
        "1",
        "--no-deps",
    ]))?)?;
    let selections = metadata["packages"]
        .as_array()
        .and_then(|packages| {
            packages
                .iter()
                .find(|package| package["name"] == "game-entry")
        })
        .and_then(|package| package["features"].as_object())
        .ok_or_else(|| io::Error::other("Cargo metadata is missing game-entry features"))?;
    if !selections.keys().any(|selection| selection != "default") {
        return Err(io::Error::other("game-entry has no game selections"));
    }
    Ok(selections
        .keys()
        .filter(|selection| *selection != "default")
        .cloned()
        .collect())
}

fn web(selection: &str) -> io::Result<()> {
    if !game_selections()?.iter().any(|name| name == selection) {
        return Err(io::Error::other(format!(
            "unknown game selection: {selection}"
        )));
    }
    if !wasm_bindgen_installed() {
        return Err(io::Error::other(
            "wasm-bindgen is missing or has the wrong version; run cargo xtask setup",
        ));
    }
    run(&mut cargo(&[
        "build",
        "-p",
        "platform-web",
        "--lib",
        "--release",
        "--locked",
        "--target",
        WEB_TARGET,
        "--target-dir",
        "target",
        "--no-default-features",
        "--features",
        &format!("game-entry/{selection}"),
    ]))?;
    let output = format!("target/web/{selection}");
    fs::create_dir_all(&output)?;
    run(Command::new("wasm-bindgen").args([
        &format!("target/{WEB_TARGET}/release/platform_web.wasm"),
        "--target",
        "web",
        "--out-dir",
        &output,
        "--out-name",
        "game",
        "--no-typescript",
    ]))?;
    fs::copy(
        "crates/platform-web/index.html",
        Path::new(&output).join("index.html"),
    )?;
    let manifest = Path::new(&output).join("assets.json");
    if !manifest.exists() {
        fs::write(manifest, "[]\n")?;
    }
    println!("==> web build: {output}/index.html");
    Ok(())
}

fn check_features() -> io::Result<()> {
    let selections = game_selections()?;
    let native_targets = TARGETS.iter().filter(|target| **target != WEB_TARGET);
    for target in native_targets {
        run(&mut cargo(&[
            "clippy",
            "--workspace",
            "--exclude",
            "platform",
            "--exclude",
            "platform-web",
            "--exclude",
            "game-entry",
            "--exclude",
            "examples",
            "--all-targets",
            "--all-features",
            "--locked",
            "--target",
            target,
        ]))?;
        run(&mut cargo(&[
            "clippy",
            "-p",
            "renderer",
            "--no-default-features",
            "--all-targets",
            "--locked",
            "--target",
            target,
        ]))?;
        for selection in &selections {
            for platform in ["cli", "linux", "cli,linux"] {
                run(&mut cargo(&[
                    "clippy",
                    "-p",
                    "platform",
                    "--no-default-features",
                    "--features",
                    &format!("{platform},game-entry/{selection}"),
                    "--all-targets",
                    "--locked",
                    "--target",
                    target,
                ]))?;
            }
        }
    }
    run(&mut cargo(&[
        "clippy",
        "-p",
        "renderer",
        "--no-default-features",
        "--lib",
        "--locked",
        "--target",
        WEB_TARGET,
    ]))?;
    for selection in &selections {
        run(&mut cargo(&[
            "clippy",
            "-p",
            "platform-web",
            "--lib",
            "--no-default-features",
            "--features",
            &format!("game-entry/{selection}"),
            "--locked",
            "--target",
            WEB_TARGET,
        ]))?;
        web(selection)?;
    }
    Ok(())
}

fn check() -> io::Result<()> {
    prerequisites()?;
    run(&mut cargo(&["fmt", "--all", "--", "--check"]))?;
    run(&mut cargo(&[
        "deny",
        "--workspace",
        "--locked",
        "check",
        "--deny",
        "warnings",
        "bans",
    ]))?;
    run(&mut cargo(&[
        "deny",
        "--manifest-path",
        "crates/asset-packer/Cargo.toml",
        "--config",
        "crates/asset-packer/deny.toml",
        "--locked",
        "check",
        "--deny",
        "warnings",
        "bans",
    ]))?;
    check_features()?;
    run(&mut cargo(&[
        "test",
        "-p",
        "game-core",
        "-p",
        "game-memory",
        "--locked",
    ]))?;
    for package in UNSAFE_CRATES {
        run(&mut miri(&[
            "test",
            "-p",
            package,
            "--all-features",
            "--locked",
        ]))?;
    }
    println!("==> all checks passed");
    Ok(())
}

fn execute() -> io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask lives in crates/xtask");
    env::set_current_dir(root)?;
    let mut arguments = env::args_os().skip(1);
    match (
        arguments.next().as_deref(),
        arguments.next(),
        arguments.next(),
    ) {
        (Some(task), None, None) if task == "setup" => setup(),
        (Some(task), None, None) if task == "check" => check(),
        (Some(task), Some(selection), None) if task == "web" => web(selection
            .to_str()
            .ok_or_else(|| io::Error::other("game selection must be UTF-8"))?),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: cargo xtask <setup|check|web GAME>",
        )),
    }
}

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}
