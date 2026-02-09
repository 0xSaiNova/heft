//! Cleanup engine.
//!
//! Executes cleanup operations based on scan results:
//! - Dry run mode (default): shows what would be deleted
//! - Execute mode: performs actual deletion with logging
//!
//! Supports:
//! - Filesystem deletions for project artifacts and caches
//! - Docker commands (docker system prune, docker image rm)
//!
//! Never deletes Docker volumes without explicit opt-in.

use std::fs;
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

    // filter entries by category if specified
    let entries_to_clean: Vec<&BloatEntry> = if let Some(cat_filter) = categories {
        result.entries.iter()
            .filter(|e| cat_filter.contains(&category_to_string(&e.category)))
            .collect()
    } else {
        result.entries.iter().collect()
    };

    for entry in entries_to_clean {
        match &mode {
            CleanMode::DryRun => {
                // just log what would be deleted
                let location_str = location_to_string(&entry.location);
                clean_result.deleted.push(format!("[dry-run] would delete: {}", location_str));
                clean_result.bytes_freed += entry.reclaimable_bytes;
            }
            CleanMode::Execute => {
                // actually delete
                match delete_entry(entry) {
                    Ok(location_str) => {
                        clean_result.deleted.push(location_str);
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
        Location::FilesystemPath(path) => {
            match fs::remove_dir_all(path) {
                Ok(_) => Ok(format!("deleted: {}", path.display())),
                Err(e) => Err(format!("failed to delete {}: {}", path.display(), e)),
            }
        }
        Location::DockerObject(obj_id) => {
            // docker objects need specific commands based on type
            // for now, just handle basic docker system prune
            let output = Command::new("docker")
                .arg("system")
                .arg("prune")
                .arg("-f")
                .arg("--filter")
                .arg(format!("label={}", obj_id))
                .output();

            match output {
                Ok(result) if result.status.success() => {
                    Ok(format!("deleted docker object: {}", obj_id))
                }
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    Err(format!("docker cleanup failed for {}: {}", obj_id, stderr))
                }
                Err(e) => {
                    Err(format!("failed to run docker command for {}: {}", obj_id, e))
                }
            }
        }
        Location::Aggregate(_) => {
            // aggregate entries are just summaries, can't be deleted directly
            Ok(String::new())
        }
    }
}

fn category_to_string(category: &BloatCategory) -> String {
    match category {
        BloatCategory::ProjectArtifacts => "project-artifacts".to_string(),
        BloatCategory::ContainerData => "container-data".to_string(),
        BloatCategory::PackageCache => "package-cache".to_string(),
        BloatCategory::IdeData => "ide-data".to_string(),
        BloatCategory::SystemCache => "system-cache".to_string(),
        BloatCategory::Other => "other".to_string(),
    }
}

fn location_to_string(location: &Location) -> String {
    match location {
        Location::FilesystemPath(path) => path.display().to_string(),
        Location::DockerObject(obj) => format!("docker:{}", obj),
        Location::Aggregate(name) => name.clone(),
    }
}
