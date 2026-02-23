//! Snapshot comparison engine.
//!
//! Compares two snapshots and reports changes:
//! - Matches entries by category and project name (not exact path)
//! - Shows per-category deltas: grew, shrank, new, gone
//! - Net change summary

use crate::scan::detector::{BloatCategory, BloatEntry};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
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
    format!("{}:{}", entry.category.as_str(), entry.name)
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
            let to_size = i64::try_from(to_entry.size_bytes).unwrap_or(i64::MAX);
            let from_size = i64::try_from(from_entry.size_bytes).unwrap_or(i64::MAX);
            let delta = to_size.saturating_sub(from_size);

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

                net_change = net_change.saturating_add(delta);
            }
        } else {
            // new entry (only in 'to' snapshot)
            let delta = i64::try_from(to_entry.size_bytes).unwrap_or(i64::MAX);

            diff_entries.push(DiffEntry {
                name: to_entry.name.clone(),
                category: to_entry.category,
                old_size: 0,
                new_size: to_entry.size_bytes,
                delta,
                diff_type: DiffType::New,
            });

            net_change = net_change.saturating_add(delta);
        }
    }

    // find gone entries (only in 'from' snapshot)
    for (key, from_entry) in &from_map {
        if !to_map.contains_key(key) {
            let delta = -i64::try_from(from_entry.size_bytes).unwrap_or(i64::MAX);

            diff_entries.push(DiffEntry {
                name: from_entry.name.clone(),
                category: from_entry.category,
                old_size: from_entry.size_bytes,
                new_size: 0,
                delta,
                diff_type: DiffType::Gone,
            });

            net_change = net_change.saturating_add(delta);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::detector::{BloatCategory, BloatEntry, Location};
    use std::path::PathBuf;

    fn entry(name: &str, size: u64) -> BloatEntry {
        BloatEntry {
            category: BloatCategory::PackageCache,
            name: name.to_string(),
            location: Location::FilesystemPath(PathBuf::from("/tmp")),
            size_bytes: size,
            reclaimable_bytes: size,
            last_modified: None,
            cleanup_hint: None,
        }
    }

    fn diff(from: &[BloatEntry], to: &[BloatEntry]) -> DiffResult {
        compare_entries(from, to, 1, 2, 0, 100)
    }

    #[test]
    fn new_entry_detected() {
        let result = diff(&[], &[entry("npm cache", 1_000_000)]);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].diff_type, DiffType::New);
        assert_eq!(result.entries[0].new_size, 1_000_000);
        assert_eq!(result.net_change, 1_000_000);
    }

    #[test]
    fn gone_entry_detected() {
        let result = diff(&[entry("npm cache", 1_000_000)], &[]);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].diff_type, DiffType::Gone);
        assert_eq!(result.entries[0].old_size, 1_000_000);
        assert_eq!(result.entries[0].new_size, 0);
        assert_eq!(result.net_change, -1_000_000);
    }

    #[test]
    fn grew_entry_detected() {
        let result = diff(
            &[entry("npm cache", 1_000_000)],
            &[entry("npm cache", 2_000_000)],
        );
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].diff_type, DiffType::Grew);
        assert_eq!(result.entries[0].delta, 1_000_000);
        assert_eq!(result.net_change, 1_000_000);
    }

    #[test]
    fn shrank_entry_detected() {
        let result = diff(
            &[entry("cargo registry", 3_000_000)],
            &[entry("cargo registry", 1_000_000)],
        );
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].diff_type, DiffType::Shrank);
        assert_eq!(result.entries[0].delta, -2_000_000);
        assert_eq!(result.net_change, -2_000_000);
    }

    #[test]
    fn unchanged_entry_not_reported() {
        let result = diff(
            &[entry("npm cache", 1_000_000)],
            &[entry("npm cache", 1_000_000)],
        );
        assert!(result.entries.is_empty());
        assert_eq!(result.net_change, 0);
    }

    #[test]
    fn net_change_mixed_operations() {
        let from = vec![entry("a", 1_000_000), entry("b", 2_000_000)];
        // a grew +500k, b is gone -2M, c is new +500k → net -1M
        let to = vec![entry("a", 1_500_000), entry("c", 500_000)];

        let result = diff(&from, &to);
        assert_eq!(result.net_change, -1_000_000);

        let types: Vec<&DiffType> = result.entries.iter().map(|e| &e.diff_type).collect();
        assert!(types.contains(&&DiffType::Grew));
        assert!(types.contains(&&DiffType::Gone));
        assert!(types.contains(&&DiffType::New));
    }

    #[test]
    fn empty_both_sides_no_entries() {
        let result = diff(&[], &[]);
        assert!(result.entries.is_empty());
        assert_eq!(result.net_change, 0);
    }

    #[test]
    fn snapshot_ids_preserved() {
        let result = compare_entries(&[], &[], 7, 13, 1000, 2000);
        assert_eq!(result.from_id, 7);
        assert_eq!(result.to_id, 13);
        assert_eq!(result.from_timestamp, 1000);
        assert_eq!(result.to_timestamp, 2000);
    }
}
