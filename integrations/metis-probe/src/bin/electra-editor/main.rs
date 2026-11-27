//! Metis native schematic editor. Run with the project snapshot path.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    windows::run()
}
#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("The pinned Metis desktop window provider requires Windows.");
    std::process::ExitCode::FAILURE
}
