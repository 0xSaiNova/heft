//! Export and display for audit results.
//!
//! Supports three output modes: summary table (default), JSON, and CSV.

use std::io::Write;

use crate::util::format_bytes;

use super::AuditResult;

/// Print a category summary table to stdout.
pub fn print_summary(result: &AuditResult) {
    println!();
    println!(
        "{:<22} {:>12} {:>12} {:>6}",
        "Category", "Size", "Files", "%"
    );
    println!("{}", "-".repeat(56));

    let mut categories: Vec<_> = result.by_category.iter().collect();
    categories.sort_by(|a, b| b.1.cmp(a.1));

    for (category, size) in &categories {
        let pct = if result.total_bytes > 0 {
            (**size as f64 / result.total_bytes as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "{:<22} {:>12} {:>12} {:>5.1}%",
            category.label(),
            format_bytes(**size),
            "", // file count per category not tracked yet
            pct
        );
    }

    println!("{}", "-".repeat(56));
    println!(
        "{:<22} {:>12} {:>12}",
        "Total",
        format_bytes(result.total_bytes),
        format!("{} files", result.file_count)
    );

    if result.inaccessible_bytes > 0 {
        println!(
            "\n{} inaccessible (permission denied)",
            format_bytes(result.inaccessible_bytes)
        );
    }

    // top directories
    if !result.top_dirs.is_empty() {
        println!(
            "\nTop {} largest directories:",
            result.top_dirs.len().min(10)
        );
        for (i, (path, size, category)) in result.top_dirs.iter().take(10).enumerate() {
            println!(
                "  {}. {:50} {:>10}  [{}]",
                i + 1,
                truncate_path(path, 50),
                format_bytes(*size),
                category.label()
            );
        }
    }

    println!(
        "\nAudit completed in {:.2}s ({} dirs, {} errors)",
        result.duration.as_secs_f64(),
        result.dir_count,
        result.errors.len()
    );
}

/// Export audit results as JSON.
pub fn export_json(result: &AuditResult, writer: &mut impl Write) -> Result<(), String> {
    let output = JsonOutput {
        total_bytes: result.total_bytes,
        file_count: result.file_count,
        dir_count: result.dir_count,
        inaccessible_bytes: result.inaccessible_bytes,
        duration_ms: result.duration.as_millis() as u64,
        categories: result
            .by_category
            .iter()
            .map(|(cat, size)| CategoryEntry {
                category: cat.label().to_string(),
                size_bytes: *size,
                percentage: if result.total_bytes > 0 {
                    (*size as f64 / result.total_bytes as f64) * 100.0
                } else {
                    0.0
                },
            })
            .collect(),
        top_dirs: result
            .top_dirs
            .iter()
            .take(20)
            .map(|(path, size, cat)| TopDirEntry {
                path: path.to_string_lossy().to_string(),
                size_bytes: *size,
                category: cat.label().to_string(),
            })
            .collect(),
    };

    serde_json::to_writer_pretty(writer, &output)
        .map_err(|e| format!("JSON serialization failed: {e}"))
}

/// Export audit results as CSV.
pub fn export_csv(result: &AuditResult, writer: &mut impl Write) -> Result<(), String> {
    writeln!(writer, "category,size_bytes,percentage")
        .map_err(|e| format!("CSV write failed: {e}"))?;

    let mut categories: Vec<_> = result.by_category.iter().collect();
    categories.sort_by(|a, b| b.1.cmp(a.1));

    for (category, size) in categories {
        let pct = if result.total_bytes > 0 {
            (*size as f64 / result.total_bytes as f64) * 100.0
        } else {
            0.0
        };
        writeln!(writer, "{},{},{:.2}", category.label(), size, pct)
            .map_err(|e| format!("CSV write failed: {e}"))?;
    }

    // top dirs section
    writeln!(writer).map_err(|e| format!("CSV write failed: {e}"))?;
    writeln!(writer, "path,size_bytes,category").map_err(|e| format!("CSV write failed: {e}"))?;
    for (path, size, cat) in result.top_dirs.iter().take(20) {
        writeln!(
            writer,
            "{},{},{}",
            path.to_string_lossy(),
            size,
            cat.label()
        )
        .map_err(|e| format!("CSV write failed: {e}"))?;
    }

    Ok(())
}

fn truncate_path(path: &std::path::Path, max_len: usize) -> String {
    let s = path.to_string_lossy();
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - max_len + 3..])
    }
}

#[derive(serde::Serialize)]
struct JsonOutput {
    total_bytes: u64,
    file_count: u64,
    dir_count: u64,
    inaccessible_bytes: u64,
    duration_ms: u64,
    categories: Vec<CategoryEntry>,
    top_dirs: Vec<TopDirEntry>,
}

#[derive(serde::Serialize)]
struct CategoryEntry {
    category: String,
    size_bytes: u64,
    percentage: f64,
}

#[derive(serde::Serialize)]
struct TopDirEntry {
    path: String,
    size_bytes: u64,
    category: String,
}
