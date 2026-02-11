//! Cleanup engine.
//!
//! Executes cleanup operations based on scan results:
//! - Dry run mode (default): shows what would be deleted
//! - Execute mode: performs actual deletion with logging
//!
//! Supports:
//! - Filesystem deletions for project artifacts and caches
//! - Docker commands for specific objects
//!
//! Never deletes Docker volumes without explicit opt-in.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::platform;
use crate::scan::{ScanResult, detector::{BloatEntry, BloatCategory, Location}};

pub enum CleanMode {
    DryRun,
    Execute,
}

pub struct CleanResult {
    pub deleted: Vec<String>,
    pub errors: Vec<String>,
    pub bytes_freed: u64,
}

pub fn run(result: &ScanResult, mode: CleanMode, categories: Option<Vec<String>>) -> CleanResult {
    let mut clean_result = CleanResult {
        deleted: Vec::new(),
        errors: Vec::new(),
        bytes_freed: 0,
    };

    // convert category filter strings to enums once for efficiency
    let category_filter: Option<Vec<BloatCategory>> = categories.map(|strings| {
        strings.iter()
            .filter_map(|s| string_to_category(s.as_str()))
            .collect()
    });

    // filter entries by category if specified, using iterator to avoid allocation
    let entries = result.entries.iter().filter(|entry| {
        // allow docker aggregates through, filter out other aggregates
        if let Location::Aggregate(ref name) = entry.location {
            if !is_docker_aggregate(name) {
                return false;
            }
        }

        if let Some(ref cats) = category_filter {
            cats.contains(&entry.category)
        } else {
            true
        }
    });

    // process based on mode - match once instead of per entry
    match mode {
        CleanMode::DryRun => {
            for entry in entries {
                let location_str = location_display(&entry.location);
                clean_result.deleted.push(format!("[dry-run] would delete: {location_str}"));
                clean_result.bytes_freed += entry.reclaimable_bytes;
            }
        }
        CleanMode::Execute => {
            for entry in entries {
                match delete_entry(entry) {
                    Ok(msg) => {
                        clean_result.deleted.push(msg);
                        clean_result.bytes_freed += entry.reclaimable_bytes;
                    }
                    Err(e) => {
                        clean_result.errors.push(e);
                    }
                }
            }
        }
    }

    clean_result
}

fn delete_entry(entry: &BloatEntry) -> Result<String, String> {
    match &entry.location {
        Location::FilesystemPath(path) => delete_filesystem_path(path),
        Location::DockerObject(obj_id) => delete_docker_object(obj_id),
        Location::Aggregate(name) => delete_docker_aggregate(name),
    }
}

fn is_docker_aggregate(name: &str) -> bool {
    matches!(
        name,
        "Images" | "Containers" | "Local Volumes" | "Build Cache"
    )
}

fn delete_filesystem_path(path: &Path) -> Result<String, String> {
    // validate path is in a safe location before deletion (issue #59)
    validate_deletion_path(path)?;

    // security: use symlink_metadata to avoid following symlinks (issue #55)
    // this also mitigates TOCTOU attacks where a directory could be replaced
    // with a symlink between scan and clean operations (issue #56)
    let metadata = fs::symlink_metadata(path).map_err(|e| {
        format!("failed to get metadata for {}: {}", path.display(), e)
    })?;

    // refuse to delete symlinks - prevents deletion of symlink targets
    // which could be anywhere on the filesystem (including system directories)
    if metadata.is_symlink() {
        return Err(format!(
            "refusing to delete symlink: {} (security: could point anywhere)",
            path.display()
        ));
    }

    // now safe to delete - we know it's not a symlink
    let result = if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };

    match result {
        Ok(_) => Ok(format!("deleted: {}", path.display())),
        Err(e) => Err(format!("failed to delete {}: {}", path.display(), e)),
    }
}

/// Validates that a path is safe to delete.
///
/// Checks:
/// - Path must be absolute
/// - Path must be under user's home directory or /tmp
/// - Path must not be the home directory itself
/// - Path must not be a system directory
fn validate_deletion_path(path: &Path) -> Result<(), String> {
    // path must be absolute
    if !path.is_absolute() {
        return Err(format!(
            "refusing to delete relative path: {} (security: must be absolute)",
            path.display()
        ));
    }

    // get canonical path to resolve any . or .. components
    let canonical = path.canonicalize().map_err(|e| {
        format!("failed to canonicalize path {}: {}", path.display(), e)
    })?;

    // check if path is under home directory
    if let Some(home) = platform::home_dir() {
        if canonical.starts_with(&home) {
            // path is under home, but make sure it's not home itself
            if canonical == home {
                return Err(format!(
                    "refusing to delete home directory: {} (security: too dangerous)",
                    canonical.display()
                ));
            }
            return Ok(());
        }
    }

    // allow /tmp and its subdirectories on unix-like systems
    #[cfg(unix)]
    {
        if canonical.starts_with("/tmp") {
            return Ok(());
        }
    }

    // allow Windows temp directories
    #[cfg(windows)]
    {
        if let Some(temp) = std::env::var_os("TEMP").or_else(|| std::env::var_os("TMP")) {
            let temp_path = PathBuf::from(temp);
            if canonical.starts_with(&temp_path) {
                return Ok(());
            }
        }
    }

    // path is not under home or temp - refuse to delete
    Err(format!(
        "refusing to delete path outside home directory: {} (security: not in safe location)",
        canonical.display()
    ))
}

fn delete_docker_object(obj_id: &str) -> Result<String, String> {
    // use docker rmi for image removal which is most common case
    let output = Command::new("docker")
        .arg("rmi")
        .arg("-f")
        .arg(obj_id)
        .output();

    match output {
        Ok(result) if result.status.success() => {
            Ok(format!("deleted docker image: {obj_id}"))
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            Err(format!("docker cleanup failed for {}: {}", obj_id, stderr.trim()))
        }
        Err(e) => {
            Err(format!("failed to run docker command for {obj_id}: {e}"))
        }
    }
}

fn delete_docker_aggregate(aggregate_type: &str) -> Result<String, String> {
    // map aggregate type to docker prune command
    let (subcommand, extra_args) = match aggregate_type {
        "Images" => ("image", vec!["prune", "-a", "-f"]),
        "Containers" => ("container", vec!["prune", "-f"]),
        "Local Volumes" => ("volume", vec!["prune", "-f"]),
        "Build Cache" => ("builder", vec!["prune", "-a", "-f"]),
        _ => return Err(format!("unknown docker aggregate type: {}", aggregate_type)),
    };

    let mut cmd = Command::new("docker");
    cmd.arg(subcommand);
    for arg in extra_args {
        cmd.arg(arg);
    }

    let output = cmd.output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            Ok(format!("cleaned docker {}: {}", aggregate_type.to_lowercase(), stdout.trim()))
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            Err(format!("docker cleanup failed for {}: {}", aggregate_type, stderr.trim()))
        }
        Err(e) => {
            Err(format!("failed to run docker command for {}: {}", aggregate_type, e))
        }
    }
}

fn string_to_category(s: &str) -> Option<BloatCategory> {
    match s {
        "project-artifacts" => Some(BloatCategory::ProjectArtifacts),
        "container-data" => Some(BloatCategory::ContainerData),
        "package-cache" => Some(BloatCategory::PackageCache),
        "ide-data" => Some(BloatCategory::IdeData),
        "system-cache" => Some(BloatCategory::SystemCache),
        "other" => Some(BloatCategory::Other),
        _ => None,
    }
}

fn location_display(location: &Location) -> String {
    match location {
        Location::FilesystemPath(path) => path.display().to_string(),
        Location::DockerObject(obj) => format!("docker:{obj}"),
        Location::Aggregate(name) => name.clone(),
    }
}
