//! SQLite snapshot storage.
//!
//! Persists scan results to a local SQLite database with two tables:
//! - snapshots: id, timestamp, disk_total, disk_used, scan_duration
//! - entries: snapshot_id, category, location, size, reclaimable
//!
//! Supports:
//! - Auto-save after every scan
//! - Listing all snapshots
//! - Loading a specific snapshot by ID

pub mod diff;

use crate::scan::ScanResult;

pub struct Snapshot {
    pub id: String,
    pub timestamp: i64,
    pub result: ScanResult,
}

pub fn save(_result: &ScanResult) -> Option<String> {
    None
}

pub fn load(_id: &str) -> Option<Snapshot> {
    None
}

pub fn list() -> Vec<Snapshot> {
    Vec::new()
}

pub fn latest() -> Option<Snapshot> {
    None
}
