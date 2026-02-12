//! JSON output for scan results.
//!
//! Serializes ScanResult to JSON for scripting and piping.

use crate::scan::ScanResult;

pub fn render(result: &ScanResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|e| {
        let error_obj = serde_json::json!({
            "error": format!("failed to serialize: {}", e)
        });
        serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_|
            r#"{"error": "catastrophic serialization failure"}"#.to_string()
        )
    })
}
