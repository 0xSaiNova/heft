use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Linux,
    Windows,
    Unknown,
}

pub fn detect() -> Platform {
    match std::env::consts::OS {
        "macos" => Platform::MacOS,
        "linux" => Platform::Linux,
        "windows" => Platform::Windows,
        _ => Platform::Unknown,
    }
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn docker_available() -> bool {
    Command::new("docker")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
