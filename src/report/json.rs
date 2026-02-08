//! JSON output for scan results.
//!
//! Serializes ScanResult to JSON for scripting and piping.

use crate::scan::ScanResult;

pub fn render(_result: &ScanResult) -> String {
    String::from("{}")
}
