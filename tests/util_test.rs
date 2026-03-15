use heft::util::{format_age, format_bytes, parse_size};

#[test]
fn format_bytes_si_units() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(999), "999 B");
    assert_eq!(format_bytes(1_000), "1.0 KB");
    assert_eq!(format_bytes(1_500), "1.5 KB");
    assert_eq!(format_bytes(1_000_000), "1.0 MB");
    assert_eq!(format_bytes(1_500_000), "1.5 MB");
    assert_eq!(format_bytes(1_000_000_000), "1.0 GB");
    assert_eq!(format_bytes(2_500_000_000), "2.5 GB");
}

#[test]
fn parse_size_round_trips() {
    // a value formatted by format_bytes should parse back to roughly the same bytes
    let original = 1_500_000_000u64; // 1.5 GB
    let formatted = format_bytes(original); // "1.5 GB"
    let parsed = parse_size(&formatted).unwrap();
    assert_eq!(parsed, original, "round-trip: {formatted} -> {parsed}");
}

#[test]
fn parse_size_various_units() {
    assert_eq!(parse_size("100B").unwrap(), 100);
    assert_eq!(parse_size("1KB").unwrap(), 1_000);
    assert_eq!(parse_size("1MB").unwrap(), 1_000_000);
    assert_eq!(parse_size("1GB").unwrap(), 1_000_000_000);
    assert_eq!(parse_size("1TB").unwrap(), 1_000_000_000_000);
}

#[test]
fn parse_size_case_insensitive() {
    assert_eq!(parse_size("1gb").unwrap(), 1_000_000_000);
    assert_eq!(parse_size("500mb").unwrap(), 500_000_000);
    assert_eq!(parse_size("100kb").unwrap(), 100_000);
}

#[test]
fn parse_size_rejects_invalid() {
    assert!(parse_size("abc").is_err());
    assert!(parse_size("").is_err());
    assert!(parse_size("1XB").is_err());
}

#[test]
fn format_age_labels() {
    assert_eq!(format_age(0), "today");
    assert_eq!(format_age(3600), "today"); // 1 hour
    assert_eq!(format_age(86400), "1d");
    assert_eq!(format_age(7 * 86400), "7d");
    assert_eq!(format_age(45 * 86400), "1mo");
    assert_eq!(format_age(400 * 86400), "1yr");
}
