//! Summary renderer for the default heft flow.

use std::collections::HashMap;

use crate::scan::detector::{BloatCategory, BloatEntry};
use crate::util::{format_age, format_bytes};

pub fn print_summary(entries: &[BloatEntry]) {
    let total_reclaimable: u64 = entries.iter().map(|e| e.reclaimable_bytes).sum();
    println!();
    println!("heft — {} reclaimable", format_bytes(total_reclaimable));
    println!();

    let mut by_category: HashMap<BloatCategory, Vec<&BloatEntry>> = HashMap::new();
    for entry in entries {
        by_category.entry(entry.category).or_default().push(entry);
    }
    let mut categories: Vec<_> = by_category.into_iter().collect();
    categories.sort_by(|a, b| {
        let a_total: u64 = a.1.iter().map(|e| e.size_bytes).sum();
        let b_total: u64 = b.1.iter().map(|e| e.size_bytes).sum();
        b_total.cmp(&a_total)
    });

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    for (category, cat_entries) in &categories {
        let cat_total: u64 = cat_entries.iter().map(|e| e.size_bytes).sum();
        let top_items: Vec<String> = cat_entries
            .iter()
            .take(3)
            .map(|e| {
                let age_str = e
                    .last_modified
                    .map(|ts| format_age(now - ts))
                    .unwrap_or_default();
                let size_str = format_bytes(e.size_bytes);
                if age_str.is_empty() {
                    format!("{} ({})", e.name, size_str)
                } else {
                    format!("{} ({}, {})", e.name, size_str, age_str)
                }
            })
            .collect();

        println!(
            "  {:<20} {:>10}   {}",
            category.label(),
            format_bytes(cat_total),
            top_items.join(" · ")
        );
    }

    let protected: Vec<&BloatEntry> = entries.iter().filter(|e| e.active == Some(true)).collect();
    if !protected.is_empty() {
        let names: Vec<&str> = protected.iter().map(|e| e.name.as_str()).collect();
        println!(
            "\n  {} active projects protected: {}",
            protected.len(),
            names.join(", ")
        );
    }
    println!();
}

pub fn print_prompt() {
    println!("  [i] Pick items to clean  [a] Clean all stale  [q] Quit");
}
