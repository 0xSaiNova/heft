//! Active-use protection for heft.
//!
//! Evaluates three signals against each discovered project root to determine
//! if a project is actively being worked on:
//! 1. Git recency (reflog timestamp or .git/index mtime)
//! 2. Source file modification (breadth-first mtime sampling)
//! 3. Running processes (cwd inspection via /proc on Linux)
//!
//! If any signal fires, the project is marked active and protected from cleanup.

pub mod git;
pub mod mtime;
pub mod process;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::scan::detector::{BloatEntry, Location};

pub struct ActivityConfig {
    pub window: Duration,
    pub sample_limit: usize,
    pub check_processes: bool,
    pub enable_git: bool,
    pub enable_mtime: bool,
    pub protected_paths: Vec<PathBuf>,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        ActivityConfig {
            window: Duration::from_secs(7 * 24 * 3600),
            sample_limit: 200,
            check_processes: true,
            enable_git: true,
            enable_mtime: true,
            protected_paths: Vec::new(),
        }
    }
}

pub struct ActivityResult {
    pub active: bool,
    pub reason: Option<String>,
}

/// Annotate scan entries with activity status.
///
/// Deduplicates by project root so multiple entries from the same project
/// (e.g. target/ and .cargo caches) share one set of signal checks.
/// Docker/Aggregate entries always get active=false.
pub fn check(entries: &[BloatEntry], config: &ActivityConfig) -> Vec<ActivityResult> {
    let now = SystemTime::now();
    let mut root_results: HashMap<PathBuf, ActivityResult> = HashMap::new();
    let mut results = Vec::with_capacity(entries.len());

    // collect unique project roots
    let mut roots: Vec<PathBuf> = Vec::new();
    for entry in entries {
        if let Location::FilesystemPath(ref path) = entry.location {
            if let Some(parent) = path.parent() {
                let root = parent.to_path_buf();
                if !roots.contains(&root) && !root_results.contains_key(&root) {
                    roots.push(root);
                }
            }
        }
    }

    // check protected paths
    for root in &roots {
        for protected in &config.protected_paths {
            if root.starts_with(protected) {
                root_results.insert(
                    root.clone(),
                    ActivityResult {
                        active: true,
                        reason: Some("protected path".to_string()),
                    },
                );
                break;
            }
        }
    }

    // check process signal for all roots at once (one /proc scan)
    let process_active = if config.check_processes {
        let unchecked: Vec<PathBuf> = roots
            .iter()
            .filter(|r| !root_results.contains_key(*r))
            .cloned()
            .collect();
        process::active_roots(&unchecked)
    } else {
        HashMap::new()
    };

    for (root, reason) in process_active {
        root_results.insert(
            root,
            ActivityResult {
                active: true,
                reason: Some(reason),
            },
        );
    }

    // check git and mtime signals for remaining unchecked roots
    for root in &roots {
        if root_results.contains_key(root) {
            continue;
        }

        // git recency
        if config.enable_git {
            if let Some(last) = git::last_activity(root) {
                if let Ok(elapsed) = now.duration_since(last) {
                    if elapsed < config.window {
                        let ago = format_duration(elapsed);
                        root_results.insert(
                            root.clone(),
                            ActivityResult {
                                active: true,
                                reason: Some(format!("git activity {ago} ago")),
                            },
                        );
                        continue;
                    }
                }
            }
        }

        // source file mtime
        if config.enable_mtime {
            if let Some(latest) = mtime::latest_source_mtime(root, config.sample_limit) {
                if let Ok(elapsed) = now.duration_since(latest) {
                    if elapsed < config.window {
                        let ago = format_duration(elapsed);
                        root_results.insert(
                            root.clone(),
                            ActivityResult {
                                active: true,
                                reason: Some(format!("source modified {ago} ago")),
                            },
                        );
                        continue;
                    }
                }
            }
        }

        // no signals fired
        root_results.insert(
            root.clone(),
            ActivityResult {
                active: false,
                reason: None,
            },
        );
    }

    // map results back to entries
    for entry in entries {
        match &entry.location {
            Location::FilesystemPath(path) => {
                if let Some(parent) = path.parent() {
                    if let Some(result) = root_results.get(parent) {
                        results.push(ActivityResult {
                            active: result.active,
                            reason: result.reason.clone(),
                        });
                    } else {
                        results.push(ActivityResult {
                            active: false,
                            reason: None,
                        });
                    }
                } else {
                    results.push(ActivityResult {
                        active: false,
                        reason: None,
                    });
                }
            }
            _ => {
                results.push(ActivityResult {
                    active: false,
                    reason: None,
                });
            }
        }
    }

    results
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}
