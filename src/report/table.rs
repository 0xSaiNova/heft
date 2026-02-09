//! TUI table rendering for scan results.
//!
//! Formats output as a categorized terminal table:
//! - Groups entries by BloatCategory
//! - Shows per-category totals and grand total
//! - Sorts by reclaimable size descending

use std::collections::HashMap;
use crate::scan::ScanResult;
use crate::scan::detector::BloatCategory;

pub fn render(result: &ScanResult) -> String {
    if result.entries.is_empty() {
        return String::from("No bloat detected.\n");
    }

    let mut output = String::new();

    // group entries by category
    let mut by_category: HashMap<BloatCategory, Vec<_>> = HashMap::new();
    for entry in &result.entries {
        by_category.entry(entry.category).or_default().push(entry);
    }

    // sort categories by total reclaimable size (largest first)
    let mut categories: Vec<_> = by_category.keys().copied().collect();
    categories.sort_by_key(|cat| {
        std::cmp::Reverse(
            by_category[cat].iter().map(|e| e.reclaimable_bytes).sum::<u64>()
        )
    });

    let mut grand_total: u64 = 0;

    for category in categories {
        let entries = &by_category[&category];
        let category_total: u64 = entries.iter().map(|e| e.reclaimable_bytes).sum();
        grand_total += category_total;

        output.push_str(&format!("\n{category:?}\n"));
        output.push_str(&"-".repeat(40));
        output.push('\n');

        // sort entries within category by reclaimable size
        let mut sorted_entries: Vec<_> = entries.iter().collect();
        sorted_entries.sort_by_key(|e| std::cmp::Reverse(e.reclaimable_bytes));

        for entry in sorted_entries {
            output.push_str(&format!(
                "  {:30} {:>10}\n",
                truncate(&entry.name, 30),
                format_bytes(entry.reclaimable_bytes)
            ));
        }

        output.push_str(&format!(
            "  {:30} {:>10}\n",
            "subtotal",
            format_bytes(category_total)
        ));
    }

    output.push_str(&format!("\n{:>42}\n", format!("TOTAL: {}", format_bytes(grand_total))));

    output
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{truncated}...")
    }
}
