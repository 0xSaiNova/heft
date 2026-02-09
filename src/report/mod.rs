pub mod table;
pub mod json;

use crate::scan::ScanResult;

pub fn print(result: &ScanResult) {
    if result.entries.is_empty() {
        println!("No bloat detected.");
    } else {
        for entry in &result.entries {
            println!(
                "{:?}: {} ({} bytes, {} reclaimable)",
                entry.category,
                entry.name,
                entry.size_bytes,
                entry.reclaimable_bytes
            );
        }
    }

    if !result.diagnostics.is_empty() {
        println!();
        for diagnostic in &result.diagnostics {
            println!("[diagnostic] {}", diagnostic);
        }
    }
}
