//! Detects build artifacts in project directories.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use walkdir::WalkDir;

use crate::config::Config;
use super::detector::{BloatCategory, BloatEntry, Detector, DetectorResult, Location};

pub struct ProjectDetector;

impl Detector for ProjectDetector {
    fn name(&self) -> &'static str {
        "projects"
    }

    fn available(&self, _config: &Config) -> bool {
        true
    }

    fn scan(&self, config: &Config) -> DetectorResult {
        let mut entries = Vec::new();
        let mut diagnostics = Vec::new();
        let mut seen_projects: HashSet<PathBuf> = HashSet::new();

        for root in &config.roots {
            if !root.exists() {
                diagnostics.push(format!("skipping {}: directory does not exist", root.display()));
                continue;
            }

            scan_directory(root, &mut entries, &mut seen_projects, &mut diagnostics);
        }

        DetectorResult { entries, diagnostics }
    }
}

fn scan_directory(
    root: &Path,
    entries: &mut Vec<BloatEntry>,
    seen_projects: &mut HashSet<PathBuf>,
    diagnostics: &mut Vec<String>,
) {
    // once we find an artifact like node_modules, we dont want to look inside it
    // for more artifacts. this set tracks what weve already claimed.
    let mut seen_artifacts: HashSet<PathBuf> = HashSet::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_hidden(e.file_name()));

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() {
            continue;
        }

        let path = entry.path();

        // already inside something we detected, skip
        if seen_artifacts.iter().any(|seen| path.starts_with(seen)) {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        if let Some(artifact) = detect_artifact(path, dir_name) {
            let project_root = path.parent().unwrap_or(path);

            // monorepos have node_modules at root and also in each package.
            // if weve seen the root already, skip the nested ones.
            if seen_projects.iter().any(|seen| project_root.starts_with(seen)) {
                seen_artifacts.insert(path.to_path_buf());
                continue;
            }

            match calculate_dir_size(path) {
                Ok((size, warnings)) => {
                    let project_name = determine_project_name(project_root, &artifact);
                    let last_modified = get_source_last_modified(project_root);

                    entries.push(BloatEntry {
                        category: BloatCategory::ProjectArtifacts,
                        name: project_name,
                        location: Location::FilesystemPath(path.to_path_buf()),
                        size_bytes: size,
                        reclaimable_bytes: size,
                        last_modified,
                        cleanup_hint: Some(artifact.cleanup_hint.to_string()),
                    });

                    seen_projects.insert(project_root.to_path_buf());
                    seen_artifacts.insert(path.to_path_buf());

                    for warning in warnings {
                        diagnostics.push(format!("{} (size may be underestimated)", warning));
                    }
                }
                Err(e) => {
                    diagnostics.push(format!("failed to calculate size of {}: {}", path.display(), e));
                }
            }
        }
    }
}

struct ArtifactType {
    cleanup_hint: &'static str,
    manifest_file: Option<&'static str>,
}

// checks if a directory is a known build artifact. returns info about how to
// clean it up and where to find the project name.
fn detect_artifact(path: &Path, dir_name: &str) -> Option<ArtifactType> {
    let parent = path.parent()?;

    match dir_name {
        "node_modules" => Some(ArtifactType {
            cleanup_hint: "safe to delete, reinstall with npm install",
            manifest_file: Some("package.json"),
        }),

        // lots of projects have a target dir, only match if theres a Cargo.toml
        "target" if parent.join("Cargo.toml").exists() => Some(ArtifactType {
            cleanup_hint: "safe to delete, rebuild with cargo build",
            manifest_file: Some("Cargo.toml"),
        }),

        // python caches show up everywhere including inside installed packages.
        // only count ones that are in actual projects, not in site-packages.
        "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".tox"
            if !is_inside_installed_packages(path) => Some(ArtifactType {
                cleanup_hint: "safe to delete, regenerated automatically",
                manifest_file: None,
            }),

        ".venv" | "venv" if has_python_project(parent) => Some(ArtifactType {
            cleanup_hint: "virtual environment, recreate with python -m venv",
            manifest_file: None,
        }),

        "vendor" if parent.join("go.mod").exists() => Some(ArtifactType {
            cleanup_hint: "safe to delete, restore with go mod vendor",
            manifest_file: Some("go.mod"),
        }),

        "vendor" if parent.join("composer.json").exists() => Some(ArtifactType {
            cleanup_hint: "safe to delete, restore with composer install",
            manifest_file: Some("composer.json"),
        }),

        ".gradle" | "build" if parent.join("build.gradle").exists() || parent.join("build.gradle.kts").exists() => {
            Some(ArtifactType {
                cleanup_hint: "safe to delete, rebuild with gradle build",
                manifest_file: None,
            })
        }

        "DerivedData" => Some(ArtifactType {
            cleanup_hint: "xcode build artifacts, safe to delete",
            manifest_file: None,
        }),

        _ => None,
    }
}

fn has_python_project(dir: &Path) -> bool {
    dir.join("requirements.txt").exists()
        || dir.join("setup.py").exists()
        || dir.join("pyproject.toml").exists()
        || dir.join("setup.cfg").exists()
}

fn is_inside_installed_packages(path: &Path) -> bool {
    path.ancestors().any(|ancestor| {
        ancestor.file_name()
            .and_then(|n| n.to_str())
            .map(|s| matches!(s, "site-packages" | "dist-packages" | "node_modules" | ".venv" | "venv"))
            .unwrap_or(false)
    })
}

// we skip hidden directories during traversal, but some artifacts we care about
// start with a dot. this returns false for those so we still find them.
fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .map(|s| {
            if !s.starts_with('.') {
                return false;
            }
            !matches!(s, ".venv" | ".pytest_cache" | ".mypy_cache" | ".tox" | ".gradle")
        })
        .unwrap_or(false)
}

fn calculate_dir_size(path: &Path) -> Result<(u64, Vec<String>), std::io::Error> {
    let mut total = 0u64;
    let mut warnings = Vec::new();
    let mut overflowed = false;

    for entry in WalkDir::new(path).follow_links(false).into_iter() {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        let file_size = metadata.len();
                        match total.checked_add(file_size) {
                            Some(new_total) => total = new_total,
                            None => {
                                if !overflowed {
                                    warnings.push("directory size exceeds u64::MAX, size capped at maximum value".to_string());
                                    overflowed = true;
                                }
                                total = u64::MAX;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if e.io_error().map(|io_err| io_err.kind() == std::io::ErrorKind::PermissionDenied).unwrap_or(false) {
                    warnings.push(format!("permission denied: {}", e.path().map(|p| p.display().to_string()).unwrap_or_else(|| "unknown path".to_string())));
                }
            }
        }
    }

    Ok((total, warnings))
}

fn determine_project_name(project_root: &Path, artifact: &ArtifactType) -> String {
    if let Some(manifest) = artifact.manifest_file {
        let manifest_path = project_root.join(manifest);
        if let Some(name) = read_project_name_from_manifest(&manifest_path) {
            return name;
        }
    }

    project_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn read_project_name_from_manifest(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let filename = path.file_name()?.to_str()?;

    match filename {
        "package.json" => extract_json_field(&content, "name"),
        "Cargo.toml" => extract_toml_package_name(&content),
        "go.mod" => content.lines().next()
            .and_then(|line| line.strip_prefix("module "))
            .map(|s| s.trim().to_string()),
        _ => None,
    }
}

// extracts a field from json using proper parsing to handle escaped characters
fn extract_json_field(content: &str, field: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(content).ok()?;
    parsed.get(field)?.as_str().map(|s| s.to_string())
}

fn extract_toml_package_name(content: &str) -> Option<String> {
    let mut in_package = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "[package]" {
            in_package = true;
            continue;
        }

        if trimmed.starts_with('[') {
            in_package = false;
            continue;
        }

        if in_package && trimmed.starts_with("name") {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let value = parts[1].trim().trim_matches('"').trim_matches('\'');
                return Some(value.to_string());
            }
        }
    }

    None
}

// finds the most recent modification time of source files in a project.
// used to identify stale projects that havent been touched in a while.
fn get_source_last_modified(project_root: &Path) -> Option<i64> {
    let mut latest: Option<SystemTime> = None;
    let source_extensions = ["rs", "js", "ts", "jsx", "tsx", "py", "go", "java", "kt", "swift"];

    // only check top few levels, dont need to go deep
    for entry in WalkDir::new(project_root)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if matches!(name, "node_modules" | "target" | ".venv" | "venv" | "vendor" | "__pycache__" | "build" | "dist") {
                continue;
            }
        }

        if entry.file_type().is_file() {
            let is_source = path.extension()
                .and_then(|e| e.to_str())
                .map(|ext| source_extensions.contains(&ext))
                .unwrap_or(false);

            if is_source {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        latest = Some(latest.map_or(modified, |l| l.max(modified)));
                    }
                }
            }
        }
    }

    latest.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}
