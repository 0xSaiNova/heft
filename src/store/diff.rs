//! Snapshot comparison engine.
//!
//! Compares two snapshots and reports changes:
//! - Matches entries by category and project name (not exact path)
//! - Shows per-category deltas: grew, shrank, new, gone
//! - Net change summary

use crate::scan::detector::{BloatEntry, BloatCategory};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum DiffType {
    Grew,
    Shrank,
    New,
    Gone,
}

#[derive(Debug, Clone)]
pub struct DiffEntry {
    pub name: String,
    pub category: BloatCategory,
    pub old_size: u64,
    pub new_size: u64,
    pub delta: i64,
    pub diff_type: DiffType,
}

pub struct DiffResult {
    pub entries: Vec<DiffEntry>,
    pub net_change: i64,
    pub from_id: i64,
    pub to_id: i64,
    pub from_timestamp: i64,
    pub to_timestamp: i64,
}

/// Create a unique key for matching entries across snapshots.
/// Uses category + name since paths can change.
fn make_key(entry: &BloatEntry) -> String {
    format!("{:?}:{}", entry.category, entry.name)
}

/// Compare two sets of entries and produce diff entries
pub fn compare_entries(
    from_entries: &[BloatEntry],
    to_entries: &[BloatEntry],
    from_id: i64,
    to_id: i64,
    from_timestamp: i64,
    to_timestamp: i64,
) -> DiffResult {
    // build lookup maps using category + name as key
    let mut from_map: HashMap<String, &BloatEntry> = HashMap::new();
    for entry in from_entries {
        from_map.insert(make_key(entry), entry);
    }

    let mut to_map: HashMap<String, &BloatEntry> = HashMap::new();
    for entry in to_entries {
        to_map.insert(make_key(entry), entry);
    }

    let mut diff_entries = Vec::new();
    let mut net_change: i64 = 0;

    // find matches, grew, and shrank
    for (key, to_entry) in &to_map {
        if let Some(from_entry) = from_map.get(key) {
            // entry exists in both snapshots
            let delta = to_entry.size_bytes as i64 - from_entry.size_bytes as i64;

            if delta != 0 {
                let diff_type = if delta > 0 {
                    DiffType::Grew
                } else {
                    DiffType::Shrank
                };

                diff_entries.push(DiffEntry {
                    name: to_entry.name.clone(),
                    category: to_entry.category,
                    old_size: from_entry.size_bytes,
                    new_size: to_entry.size_bytes,
                    delta,
                    diff_type,
                });

                net_change += delta;
            }
        } else {
            // new entry (only in 'to' snapshot)
            let delta = to_entry.size_bytes as i64;

            diff_entries.push(DiffEntry {
                name: to_entry.name.clone(),
                category: to_entry.category,
                old_size: 0,
                new_size: to_entry.size_bytes,
                delta,
                diff_type: DiffType::New,
            });

            net_change += delta;
        }
    }

    // find gone entries (only in 'from' snapshot)
    for (key, from_entry) in &from_map {
        if !to_map.contains_key(key) {
            let delta = -(from_entry.size_bytes as i64);

            diff_entries.push(DiffEntry {
                name: from_entry.name.clone(),
                category: from_entry.category,
                old_size: from_entry.size_bytes,
                new_size: 0,
                delta,
                diff_type: DiffType::Gone,
            });

            net_change += delta;
        }
    }

    DiffResult {
        entries: diff_entries,
        net_change,
        from_id,
        to_id,
        from_timestamp,
        to_timestamp,
    }
}
