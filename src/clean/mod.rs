//! Cleanup engine.
//!
//! Executes cleanup operations based on scan results:
//! - Dry run mode (default): shows what would be deleted
//! - Execute mode: performs actual deletion with logging
//! - Interactive mode: asks for confirmation per category
//!
//! Supports:
//! - Filesystem deletions for project artifacts and caches
//! - Docker commands (docker system prune, docker image rm)
//!
//! Never deletes Docker volumes without explicit opt-in.

use crate::scan::ScanResult;

pub enum CleanMode {
    DryRun,
    Execute,
}

pub struct CleanResult {
    pub deleted: Vec<String>,
    pub errors: Vec<String>,
    pub bytes_freed: u64,
}

pub fn run(_result: &ScanResult, _mode: CleanMode) -> CleanResult {
    CleanResult {
        deleted: Vec::new(),
        errors: Vec::new(),
        bytes_freed: 0,
    }
}
