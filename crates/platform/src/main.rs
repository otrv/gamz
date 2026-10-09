#[cfg(not(target_os = "linux"))]
compile_error!("the platform layer is implemented only for Linux");

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
mod memory;

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    match linux::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gamz: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
