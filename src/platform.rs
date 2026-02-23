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

/// Returns true when heft is running inside a WSL2 environment.
/// WSL_INTEROP is set exclusively by WSL2 (not WSL1) and points to the
/// interop socket. WSL_DISTRO_NAME is set by both WSL1 and WSL2.
pub fn is_wsl() -> bool {
    std::env::var_os("WSL_INTEROP").is_some()
}
