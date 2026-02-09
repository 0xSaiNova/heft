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
        // skip aggregate entries early, they're just summaries
        if matches!(entry.location, Location::Aggregate(_)) {
            return false;
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
                clean_result.deleted.push(format!("[dry-run] would delete: {}", location_str));
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
        Location::Aggregate(_) => unreachable!("aggregates filtered before deletion"),
    }
}

fn delete_filesystem_path(path: &Path) -> Result<String, String> {
    // handle both files and directories
    let result = if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };

    match result {
        Ok(_) => Ok(format!("deleted: {}", path.display())),
        Err(e) => Err(format!("failed to delete {}: {}", path.display(), e)),
    }
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
            Ok(format!("deleted docker image: {}", obj_id))
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            Err(format!("docker cleanup failed for {}: {}", obj_id, stderr.trim()))
        }
        Err(e) => {
            Err(format!("failed to run docker command for {}: {}", obj_id, e))
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
        Location::DockerObject(obj) => format!("docker:{}", obj),
        Location::Aggregate(name) => name.clone(),
    }
}
