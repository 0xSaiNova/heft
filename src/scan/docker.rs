//! Docker storage detector.
//!
//! Queries the Docker daemon via `docker system df --format json` for:
//! - Images (total and reclaimable)
//! - Containers (total and reclaimable)
//! - Volumes (total and reclaimable)
//! - Build cache (total and reclaimable)
//!
//! Also detects Docker Desktop VM disk images on macOS and Windows:
//! - macOS: ~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw
//! - Windows: %USERPROFILE%\AppData\Local\Docker\wsl\data\ext4.vhdx
//!
//! Handles gracefully:
//! - Docker not installed
//! - Docker daemon not running
//! - Permission denied
//!
//! Does not walk Docker's internal storage directories directly.

use std::process::Command;
use std::fs;
use serde::Deserialize;

use crate::config::Config;
use crate::platform;
use super::detector::{Detector, DetectorResult, BloatEntry, BloatCategory, Location};

pub struct DockerDetector;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DockerDfEntry {
    #[serde(rename = "Type")]
    type_: String,
    size: String,
    reclaimable: String,
}

impl Detector for DockerDetector {
    fn name(&self) -> &'static str {
        "docker"
    }

    fn available(&self, config: &Config) -> bool {
        !config.skip_docker && platform::docker_available()
    }

    fn scan(&self, config: &Config) -> DetectorResult {
        let mut all_entries = Vec::new();
        let mut diagnostics = Vec::new();

        // get docker API resources (images, containers, volumes, build cache)
        match run_docker_system_df(config) {
            Ok(mut entries) => all_entries.append(&mut entries),
            Err(e) => diagnostics.push(e),
        }

        // detect Docker Desktop VM disk images (macOS/Windows only)
        if let Some(vm_entry) = detect_docker_desktop_vm(config) {
            all_entries.push(vm_entry);
        }

        DetectorResult {
            entries: all_entries,
            diagnostics,
        }
    }
}

fn run_docker_system_df(config: &Config) -> Result<Vec<BloatEntry>, String> {
    let output = Command::new("docker")
        .arg("system")
        .arg("df")
        .arg("--format")
        .arg("json")
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return Err(format!("docker: failed to run command: {}", e));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // check for common error patterns
        if stderr.contains("Cannot connect to the Docker daemon")
            || stderr.contains("Is the docker daemon running") {
            return Err("docker: daemon not running (start Docker Desktop or dockerd)".to_string());
        }

        if stderr.contains("permission denied") || stderr.contains("EACCES") {
            return Err("docker: permission denied (add user to docker group or run with sudo)".to_string());
        }

        return Err(format!("docker: command failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    // docker system df outputs JSONL (one JSON object per line)
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let df_entry: DockerDfEntry = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(e) => {
                if config.verbose {
                    return Err(format!("docker: failed to parse output: {}", e));
                }
                continue;
            }
        };

        let size_bytes = parse_docker_size(&df_entry.size)?;
        let reclaimable_bytes = parse_docker_size(&df_entry.reclaimable)?;

        // only create entries for types that have actual data
        if size_bytes == 0 {
            continue;
        }

        let name = match df_entry.type_.as_str() {
            "Images" => "docker images",
            "Containers" => "docker containers",
            "Local Volumes" => "docker volumes",
            "Build Cache" => "docker build cache",
            other => other,
        };

        entries.push(BloatEntry {
            category: BloatCategory::ContainerData,
            name: name.to_string(),
            location: Location::Aggregate(df_entry.type_.clone()),
            size_bytes,
            reclaimable_bytes,
            last_modified: None,
            cleanup_hint: Some(get_cleanup_hint(&df_entry.type_)),
        });
    }

    Ok(entries)
}

fn parse_docker_size(size_str: &str) -> Result<u64, String> {
    // docker sizes look like "8.056GB", "248.1MB (3%)", "0B"
    // extract just the size part before any parenthesis
    let size_part = size_str.split('(').next().unwrap_or(size_str).trim();

    if size_part == "0B" || size_part.is_empty() {
        return Ok(0);
    }

    // find where the number ends and unit begins
    let mut num_end = 0;
    for (i, c) in size_part.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            num_end = i + 1;
        } else {
            break;
        }
    }

    if num_end == 0 {
        return Err(format!("docker: invalid size format: {}", size_str));
    }

    let num_str = &size_part[..num_end];
    let unit = size_part[num_end..].trim();

    let num: f64 = num_str.parse()
        .map_err(|_| format!("docker: invalid number in size: {}", size_str))?;

    let multiplier: u64 = match unit {
        "B" => 1,
        "kB" | "KB" => 1_000,
        "MB" => 1_000_000,
        "GB" => 1_000_000_000,
        "TB" => 1_000_000_000_000,
        "KiB" => 1_024,
        "MiB" => 1_048_576,
        "GiB" => 1_073_741_824,
        "TiB" => 1_099_511_627_776,
        _ => return Err(format!("docker: unknown size unit: {}", unit)),
    };

    Ok((num * multiplier as f64) as u64)
}

fn get_cleanup_hint(type_: &str) -> String {
    match type_ {
        "Images" => "docker image prune -a".to_string(),
        "Containers" => "docker container prune".to_string(),
        "Local Volumes" => "docker volume prune".to_string(),
        "Build Cache" => "docker builder prune".to_string(),
        _ => "docker system prune".to_string(),
    }
}

/// Detect Docker Desktop VM disk image on macOS and Windows.
///
/// NOTE: macOS and Windows detection paths are UNTESTED on actual hardware.
/// Paths are based on Docker Desktop documentation. If you're running this on
/// macOS or Windows and encounter issues, please report at:
/// https://github.com/0xSaiNova/heft/issues/42
///
/// These VM disk images can be 30-60 GB and don't automatically shrink when
/// you delete containers/images inside the VM.
fn detect_docker_desktop_vm(config: &Config) -> Option<BloatEntry> {
    let platform = platform::detect();

    // only macOS and Windows use VM disk images for Docker Desktop
    let (vm_path, cleanup_hint) = match platform {
        platform::Platform::MacOS => {
            // UNTESTED: This path is based on Docker Desktop documentation
            let home = platform::home_dir()?;
            let path = home.join("Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw");
            let hint = "Docker Desktop VM disk. To reclaim space: Docker Desktop → Settings → Resources → Disk image size → 'Clean/Purge data'. Or run 'docker system prune' and restart Docker Desktop.".to_string();
            (path, hint)
        }
        platform::Platform::Windows => {
            // UNTESTED: This path is based on Docker Desktop WSL2 documentation
            let home = platform::home_dir()?;
            let path = home.join("AppData/Local/Docker/wsl/data/ext4.vhdx");
            let hint = "Docker Desktop VM disk. To reclaim space: run 'wsl --shutdown' then 'Optimize-VHD -Path $path -Mode Full' in PowerShell (admin).".to_string();
            (path, hint)
        }
        _ => return None, // Linux doesn't use VM disk images
    };

    // check if the VM disk file exists
    if !vm_path.exists() {
        if config.verbose {
            eprintln!("docker: VM disk not found at {}", vm_path.display());
        }
        return None;
    }

    // get the file size
    let metadata = match fs::metadata(&vm_path) {
        Ok(m) => m,
        Err(e) => {
            if config.verbose {
                eprintln!("docker: failed to get VM disk metadata: {}", e);
            }
            return None;
        }
    };

    let size_bytes = metadata.len();

    // only report if there's actual data
    if size_bytes == 0 {
        return None;
    }

    Some(BloatEntry {
        category: BloatCategory::ContainerData,
        name: format!("Docker Desktop VM ({})", vm_path.display()),
        location: Location::FilesystemPath(vm_path),
        size_bytes,
        reclaimable_bytes: 0, // we can't determine reclaimable size without analyzing the VM
        last_modified: None, // timestamp not needed for VM disk
        cleanup_hint: Some(cleanup_hint),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_docker_size() {
        assert_eq!(parse_docker_size("0B").unwrap(), 0);
        assert_eq!(parse_docker_size("1kB").unwrap(), 1_000);
        assert_eq!(parse_docker_size("1.5MB").unwrap(), 1_500_000);

        // floating point precision causes small differences, allow 1 byte variance
        let gb_result = parse_docker_size("8.056GB").unwrap();
        assert!((gb_result as i64 - 8_056_000_000).abs() <= 1);

        assert_eq!(parse_docker_size("248.1MB (3%)").unwrap(), 248_100_000);
        assert_eq!(parse_docker_size("141.8MB").unwrap(), 141_800_000);
        assert_eq!(parse_docker_size("27.57MB").unwrap(), 27_570_000);
        assert_eq!(parse_docker_size("578.6kB (2%)").unwrap(), 578_600);
    }
}
