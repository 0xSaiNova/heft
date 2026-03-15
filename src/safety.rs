//! Safety tier classification for bloat entries.
//!
//! Every entry gets a tier that determines picker behavior and --auto eligibility.
//! Tier 1 (Disposable) and Tier 2 (Rebuildable) can be auto-selected when stale.
//! Tier 3 (Caution) and Tier 4 (UserData) require manual selection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::scan::detector::{BloatCategory, BloatEntry, Location};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyTier {
    Disposable,
    Rebuildable,
    Caution,
    UserData,
}

impl SafetyTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyTier::Disposable => "disposable",
            SafetyTier::Rebuildable => "rebuildable",
            SafetyTier::Caution => "caution",
            SafetyTier::UserData => "user_data",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "disposable" => Some(SafetyTier::Disposable),
            "rebuildable" => Some(SafetyTier::Rebuildable),
            "caution" => Some(SafetyTier::Caution),
            "user_data" => Some(SafetyTier::UserData),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SafetyTier::Disposable | SafetyTier::Rebuildable => "safe",
            SafetyTier::Caution => "caution",
            SafetyTier::UserData => "user data",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyInfo {
    pub tier: SafetyTier,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_dirty_files: Option<u32>,
}

/// check if a git repo has uncommitted changes.
/// returns None if not a git repo or git unavailable.
/// returns Some(0) if clean, Some(n) if n files changed.
pub fn check_git_status(project_root: &Path) -> Option<u32> {
    if !project_root.join(".git").exists() {
        return None;
    }

    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().filter(|l| !l.is_empty()).count() as u32;
    Some(count)
}

/// shared classification for project artifacts given a pre-fetched git status.
fn classify_project_artifact(git_status: Option<u32>) -> SafetyInfo {
    match git_status {
        Some(0) => SafetyInfo {
            tier: SafetyTier::Rebuildable,
            reason: "git clean, safe to rebuild".into(),
            git_dirty_files: Some(0),
        },
        Some(n) => SafetyInfo {
            tier: SafetyTier::Caution,
            reason: format!("{} uncommitted change{}", n, if n == 1 { "" } else { "s" }),
            git_dirty_files: Some(n),
        },
        None => SafetyInfo {
            tier: SafetyTier::Caution,
            reason: "no git repo, cannot verify source".into(),
            git_dirty_files: None,
        },
    }
}

/// classify a single entry. for ProjectArtifacts this shells out to git,
/// so prefer classify_all for bulk classification (deduplicates git calls).
pub fn classify_safety(entry: &BloatEntry) -> SafetyInfo {
    match entry.category {
        BloatCategory::PackageCache | BloatCategory::SystemCache => SafetyInfo {
            tier: SafetyTier::Disposable,
            reason: "cache, re-downloads on demand".into(),
            git_dirty_files: None,
        },
        BloatCategory::ProjectArtifacts => {
            let project_root = match &entry.location {
                Location::FilesystemPath(p) => p.parent().map(|p| p.to_path_buf()),
                _ => None,
            };
            match project_root {
                Some(root) => classify_project_artifact(check_git_status(&root)),
                None => SafetyInfo {
                    tier: SafetyTier::Caution,
                    reason: "cannot determine project root".into(),
                    git_dirty_files: None,
                },
            }
        }
        BloatCategory::ContainerData => SafetyInfo {
            tier: SafetyTier::Caution,
            reason: "container data, may contain unpushed images".into(),
            git_dirty_files: None,
        },
        BloatCategory::IdeData => SafetyInfo {
            tier: SafetyTier::Caution,
            reason: "IDE data, may contain local configuration".into(),
            git_dirty_files: None,
        },
        BloatCategory::LargeFile => SafetyInfo {
            tier: SafetyTier::UserData,
            reason: "user file, cannot be automatically recreated".into(),
            git_dirty_files: None,
        },
        BloatCategory::Other => SafetyInfo {
            tier: SafetyTier::Caution,
            reason: "unknown provenance".into(),
            git_dirty_files: None,
        },
    }
}

/// classify all entries in bulk, deduplicating git status checks per project root.
pub fn classify_all(entries: &mut [BloatEntry]) {
    let mut git_cache: HashMap<PathBuf, Option<u32>> = HashMap::new();

    for entry in entries.iter_mut() {
        let info = match entry.category {
            BloatCategory::ProjectArtifacts => {
                let project_root = match &entry.location {
                    Location::FilesystemPath(p) => p.parent().map(|p| p.to_path_buf()),
                    _ => None,
                };
                match project_root {
                    Some(root) => {
                        let status = git_cache
                            .entry(root.clone())
                            .or_insert_with(|| check_git_status(&root));
                        classify_project_artifact(*status)
                    }
                    None => SafetyInfo {
                        tier: SafetyTier::Caution,
                        reason: "cannot determine project root".into(),
                        git_dirty_files: None,
                    },
                }
            }
            _ => classify_safety(entry),
        };
        entry.safety = Some(info);
    }
}

/// whether an entry should be pre-selected in the picker or included in --auto.
/// only Tier 1 (Disposable) and Tier 2 (Rebuildable) stale items qualify.
pub fn should_preselect(entry: &BloatEntry, include_active: bool) -> bool {
    if entry.active == Some(true) && !include_active {
        return false;
    }
    if entry.staleness_score.unwrap_or(0.0) <= 0.0 {
        return false;
    }
    matches!(
        entry.safety.as_ref().map(|s| &s.tier),
        Some(SafetyTier::Disposable) | Some(SafetyTier::Rebuildable)
    )
}
