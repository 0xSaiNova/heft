//! Full-drive audit command.
//!
//! Categorizes all disk usage on a drive or subtree using a parallel filesystem
//! walker. Results can be viewed interactively via TUI, exported as JSON/CSV,
//! or saved to SQLite for historical comparison.

pub mod categories;
pub mod export;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use categories::{AuditCategory, CustomRule};

use serde::Serialize;

/// Configuration for the audit command.
pub struct AuditConfig {
    pub roots: Vec<PathBuf>,
    pub cross_mount: bool,
    pub min_entry_size: u64,
    pub custom_rules: Vec<CustomRule>,
    pub export: Option<String>,
    pub save: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        AuditConfig {
            roots: Vec::new(),
            cross_mount: false,
            min_entry_size: 10 * 1024 * 1024, // 10 MB
            custom_rules: Vec::new(),
            export: None,
            save: false,
        }
    }
}

/// A categorized disk usage entry.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub category: AuditCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime: Option<i64>,
}

/// Results from a full audit run.
pub struct AuditResult {
    pub by_category: HashMap<AuditCategory, u64>,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub inaccessible_bytes: u64,
    pub duration: Duration,
    pub errors: Vec<String>,
    pub top_dirs: Vec<(PathBuf, u64, AuditCategory)>,
}

/// Pseudo-filesystems that should always be skipped during audit.
const SKIP_PATHS: &[&str] = &["/proc", "/sys", "/dev", "/run"];

fn should_skip_path(path: &Path) -> bool {
    SKIP_PATHS.iter().any(|p| path.starts_with(p))
}

/// Run a full disk audit.
pub fn run(config: &AuditConfig) -> AuditResult {
    let start = std::time::Instant::now();
    let home = crate::platform::home_dir();
    let mut by_category: HashMap<AuditCategory, u64> = HashMap::new();
    let mut total_bytes: u64 = 0;
    let mut file_count: u64 = 0;
    let mut dir_count: u64 = 0;
    let mut inaccessible_bytes: u64 = 0;
    let mut errors: Vec<String> = Vec::new();
    let mut dir_sizes: HashMap<PathBuf, (u64, AuditCategory)> = HashMap::new();

    for root in &config.roots {
        // get the root device ID for mount boundary filtering
        #[cfg(unix)]
        let root_dev = if !config.cross_mount {
            use std::os::unix::fs::MetadataExt;
            std::fs::metadata(root).ok().map(|m| m.dev())
        } else {
            None
        };

        let walker = jwalk::WalkDir::new(root).follow_links(false).sort(true);

        for entry in walker {
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    // skip pseudo-filesystems (always, regardless of --cross-mount)
                    if should_skip_path(&path) {
                        continue;
                    }

                    // skip entries on different filesystems unless --cross-mount
                    #[cfg(unix)]
                    if let Some(root_dev) = root_dev {
                        if let Ok(meta) = entry.metadata() {
                            use std::os::unix::fs::MetadataExt;
                            if meta.dev() != root_dev {
                                continue;
                            }
                        }
                    }

                    let file_type = entry.file_type;

                    if file_type.is_file() {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                        let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        let ext = path.extension().and_then(|e| e.to_str());

                        let (category, _subcategory) = categories::classify_path(
                            &path,
                            dir_name,
                            ext,
                            home.as_deref(),
                            &config.custom_rules,
                        );

                        *by_category.entry(category).or_insert(0) += size;
                        total_bytes += size;
                        file_count += 1;

                        // track parent directory size for top dirs
                        if let Some(parent) = path.parent() {
                            let entry = dir_sizes
                                .entry(parent.to_path_buf())
                                .or_insert((0, category));
                            entry.0 += size;
                        }
                    } else if file_type.is_dir() {
                        dir_count += 1;
                    }
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("Permission denied") {
                        inaccessible_bytes += 4096;
                    }
                    errors.push(msg);
                }
            }
        }
    }

    // find top directories by size
    let mut top_dirs: Vec<(PathBuf, u64, AuditCategory)> = dir_sizes
        .into_iter()
        .filter(|(_, (size, _))| *size >= config.min_entry_size)
        .map(|(path, (size, cat))| (path, size, cat))
        .collect();
    top_dirs.sort_by(|a, b| b.1.cmp(&a.1));
    top_dirs.truncate(20);

    AuditResult {
        by_category,
        total_bytes,
        file_count,
        dir_count,
        inaccessible_bytes,
        duration: start.elapsed(),
        errors,
        top_dirs,
    }
}
