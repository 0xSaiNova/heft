//! Detects build artifacts in project directories.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use walkdir::WalkDir;

use super::detector::{BloatCategory, BloatEntry, Detector, DetectorResult, Location};
use crate::config::Config;

pub struct ProjectDetector;

impl Detector for ProjectDetector {
    fn name(&self) -> &'static str {
        "projects"
    }

    fn available(&self, config: &Config) -> bool {
        config.is_detector_enabled("projects")
    }

    fn scan(&self, config: &Config) -> DetectorResult {
        let mut entries = Vec::new();
        let mut diagnostics = Vec::new();
        let mut seen_projects: HashSet<PathBuf> = HashSet::new();

        for root in &config.roots {
            if !root.exists() {
                diagnostics.push(format!(
                    "skipping {}: directory does not exist",
                    root.display()
                ));
                continue;
            }

            scan_directory(root, &mut entries, &mut seen_projects, &mut diagnostics);
        }

        DetectorResult {
            entries,
            diagnostics,
        }
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
            if seen_projects
                .iter()
                .any(|seen| project_root.starts_with(seen))
            {
                seen_artifacts.insert(path.to_path_buf());
                continue;
            }

            match super::calculate_dir_size(path) {
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
                        diagnostics.push(format!("{warning} (size may be underestimated)"));
                    }
                }
                Err(e) => {
                    diagnostics.push(format!(
                        "failed to calculate size of {}: {}",
                        path.display(),
                        e
                    ));
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
            if !is_inside_installed_packages(path) =>
        {
            Some(ArtifactType {
                cleanup_hint: "safe to delete, regenerated automatically",
                manifest_file: None,
            })
        }

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

        ".gradle"
            if parent.join("build.gradle").exists() || parent.join("build.gradle.kts").exists() =>
        {
            Some(ArtifactType {
                cleanup_hint: "safe to delete, rebuild with gradle build",
                manifest_file: None,
            })
        }

        // only flag "build" dirs as gradle if they actually contain gradle artifacts
        // prevents false positives on legitimate build folders used for other purposes
        "build"
            if (parent.join("build.gradle").exists()
                || parent.join("build.gradle.kts").exists())
                && is_gradle_build_dir(path) =>
        {
            Some(ArtifactType {
                cleanup_hint: "safe to delete, rebuild with gradle build",
                manifest_file: None,
            })
        }

        // only flag DerivedData if it's actually from xcode
        // check for xcode markers or being in the xcode cache location
        "DerivedData" if is_xcode_derived_data(path, parent) => Some(ArtifactType {
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
        ancestor
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| {
                matches!(
                    s,
                    "site-packages" | "dist-packages" | "node_modules" | ".venv" | "venv"
                )
            })
            .unwrap_or(false)
    })
}

// verify a "build" directory actually contains gradle artifacts, not just any folder named build.
// checks for typical gradle output directories to avoid false positives.
fn is_gradle_build_dir(path: &Path) -> bool {
    path.join("classes").exists()
        || path.join("libs").exists()
        || path.join("tmp").exists()
        || path.join("generated").exists()
        || path.join("intermediates").exists()
}

// verify a "DerivedData" directory is actually from xcode, not just any folder with that name.
// checks for xcode-specific markers or being in the standard xcode cache location.
fn is_xcode_derived_data(path: &Path, parent: &Path) -> bool {
    // check if in standard xcode cache location (~/Library/Developer/Xcode/DerivedData)
    // fixed: properly check if ancestor named "Xcode" has parent named "Developer"
    let in_xcode_cache = path.ancestors().any(|ancestor| {
        if let (Some(name), Some(parent)) = (ancestor.file_name(), ancestor.parent()) {
            name.to_str() == Some("Xcode")
                && parent
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s == "Developer")
                    .unwrap_or(false)
        } else {
            false
        }
    });

    // check for xcode project file in parent directories.
    // bounded to home directory and capped at 10 levels to avoid walking
    // all the way up to / and calling read_dir on every ancestor.
    let home = crate::platform::home_dir();
    let has_xcode_project = parent
        .ancestors()
        .take_while(|ancestor| home.as_deref().map(|h| *ancestor != h).unwrap_or(true))
        .take(10)
        .any(|ancestor| {
            if let Ok(entries) = std::fs::read_dir(ancestor) {
                entries.flatten().any(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|s| s == "xcodeproj" || s == "xcworkspace")
                        .unwrap_or(false)
                })
            } else {
                false
            }
        });

    // check for xcode-specific subdirectories in DerivedData
    let has_xcode_markers = path.join("Build").exists()
        || path.join("Logs").exists()
        || path.join("ModuleCache").exists()
        || path.join("info.plist").exists();

    in_xcode_cache || has_xcode_project || has_xcode_markers
}

// we skip hidden directories during traversal, but some artifacts we care about
// start with a dot. this returns false for those so we still find them.
fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .map(|s| {
            if !s.starts_with('.') {
                return false;
            }
            !matches!(
                s,
                ".venv" | ".pytest_cache" | ".mypy_cache" | ".tox" | ".gradle"
            )
        })
        .unwrap_or(false)
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
    // check file size before reading to prevent OOM on maliciously large files
    const MAX_MANIFEST_SIZE: u64 = 1024 * 1024; // 1MB
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_MANIFEST_SIZE {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;
    let filename = path.file_name()?.to_str()?;

    match filename {
        "package.json" => extract_json_field(&content, "name"),
        "Cargo.toml" => extract_toml_package_name(&content),
        "go.mod" => content
            .lines()
            .next()
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

        // exit package section only when encountering a DIFFERENT section
        // this allows [dependencies] and other sections after [package] without breaking
        if trimmed.starts_with('[') && trimmed != "[package]" {
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
    let source_extensions = [
        "rs", "js", "ts", "jsx", "tsx", "py", "go", "java", "kt", "swift",
    ];

    // only check top few levels, dont need to go deep.
    // filter_entry prunes descent into artifact directories entirely, not just
    // skips their entry. using continue here would skip the entry but still
    // let walkdir descend into it (scanning thousands of files for nothing).
    for entry in WalkDir::new(project_root)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if !e.file_type().is_dir() {
                return true;
            }
            e.file_name()
                .to_str()
                .map(|s| {
                    !matches!(
                        s,
                        "node_modules"
                            | "target"
                            | ".venv"
                            | "venv"
                            | "vendor"
                            | "__pycache__"
                            | "build"
                            | "dist"
                    )
                })
                .unwrap_or(true)
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let is_source = entry
                .path()
                .extension()
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

    latest
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}
