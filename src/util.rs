//! Shared utility functions

/// Format bytes into human-readable sizes (B, KB, MB, GB)
pub fn format_bytes(bytes: u64) -> String {
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

/// Parse size strings like "500MB", "1GB", "100KB" into bytes (1000 based)
pub fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let split_pos = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
    let (num_part, unit) = s.split_at(split_pos);
    let num: f64 = num_part
        .trim()
        .parse()
        .map_err(|_| format!("invalid size: {s}"))?;
    let multiplier: u64 = match unit.to_uppercase().as_str() {
        "" | "B" => 1,
        "KB" | "K" => 1_000,
        "MB" | "M" => 1_000_000,
        "GB" | "G" => 1_000_000_000,
        "TB" | "T" => 1_000_000_000_000,
        other => return Err(format!("unknown size unit: {other}")),
    };
    Ok((num * multiplier as f64) as u64)
}

/// Format seconds age into human readable string
pub fn format_age(age_secs: i64) -> String {
    let days = age_secs / 86400;
    if days < 1 {
        return "today".to_string();
    }
    if days < 30 {
        return format!("{days}d");
    }
    if days < 365 {
        return format!("{}mo", days / 30);
    }
    format!("{}yr", days / 365)
}
