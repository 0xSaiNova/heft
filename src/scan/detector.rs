use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BloatCategory {
    ProjectArtifacts,
    ContainerData,
    PackageCache,
    IdeData,
    SystemCache,
    Other,
    LargeFile,
}

impl BloatCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            BloatCategory::ProjectArtifacts => "ProjectArtifacts",
            BloatCategory::ContainerData => "ContainerData",
            BloatCategory::PackageCache => "PackageCache",
            BloatCategory::IdeData => "IdeData",
            BloatCategory::SystemCache => "SystemCache",
            BloatCategory::Other => "Other",
            BloatCategory::LargeFile => "LargeFile",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            BloatCategory::ProjectArtifacts => "Project Artifacts",
            BloatCategory::ContainerData => "Container Data",
            BloatCategory::PackageCache => "Package Cache",
            BloatCategory::IdeData => "IDE Data",
            BloatCategory::SystemCache => "System Cache",
            BloatCategory::Other => "Other",
            BloatCategory::LargeFile => "Large Files",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Location {
    FilesystemPath(PathBuf),
    DockerObject(String),
    Aggregate(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloatEntry {
    pub category: BloatCategory,
    pub name: String,
    pub location: Location,
    pub size_bytes: u64,
    pub reclaimable_bytes: u64,
    pub last_modified: Option<i64>,
    pub cleanup_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staleness_score: Option<f64>,
}

/// Source file extensions used for activity detection and last-modified scanning.
pub const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "js", "ts", "jsx", "tsx", "py", "go", "java", "kt", "swift", "cs", "fs", "vb",
];

/// Known artifact directory names that should be skipped during source file walks.
pub const ARTIFACT_DIR_NAMES: &[&str] = &[
    "node_modules",
    "target",
    ".venv",
    "venv",
    "vendor",
    "__pycache__",
    "build",
    "dist",
    ".gradle",
    "bin",
    "obj",
    "DerivedData",
];

pub struct DetectorResult {
    pub entries: Vec<BloatEntry>,
    pub diagnostics: Vec<String>,
}

impl DetectorResult {
    pub fn empty() -> Self {
        DetectorResult {
            entries: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn with_diagnostic(message: String) -> Self {
        DetectorResult {
            entries: Vec::new(),
            diagnostics: vec![message],
        }
    }
}

pub trait Detector {
    fn name(&self) -> &'static str;
    fn available(&self, config: &Config) -> bool;
    fn scan(&self, config: &Config) -> DetectorResult;
}
