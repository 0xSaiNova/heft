//! JSON output for scan results.
//!
//! Serializes ScanResult to JSON for scripting and piping.

use crate::scan::ScanResult;

pub fn render(result: &ScanResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|e| {
        format!("{{\"error\": \"failed to serialize: {e}\"}}")
    })
}
