use std::path::PathBuf;
use serde::{Serialize, Deserialize};

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BloatCategory {
    ProjectArtifacts,
    ContainerData,
    PackageCache,
    IdeData,
    SystemCache,
    Other,
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
}

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
