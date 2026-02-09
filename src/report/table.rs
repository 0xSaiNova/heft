//! TUI table rendering for scan results.
//!
//! Formats output as a categorized terminal table:
//! - Groups entries by BloatCategory
//! - Shows per-category totals and grand total
//! - Sorts by reclaimable size descending
//! - Compact layout, color only if terminal supports it

use crate::scan::ScanResult;

pub fn render(_result: &ScanResult) -> String {
    String::new()
}
