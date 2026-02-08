//! Snapshot comparison engine.
//!
//! Compares two snapshots and reports changes:
//! - Matches entries by category and project name (not exact path)
//! - Shows per-category deltas: grew, shrank, new, gone
//! - Net change summary

use super::Snapshot;

pub struct DiffEntry {
    pub name: String,
    pub category: String,
    pub old_size: u64,
    pub new_size: u64,
    pub delta: i64,
}

pub struct DiffResult {
    pub entries: Vec<DiffEntry>,
    pub net_change: i64,
}

pub fn compare(_from: &Snapshot, _to: &Snapshot) -> DiffResult {
    DiffResult {
        entries: Vec::new(),
        net_change: 0,
    }
}
