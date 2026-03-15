use heft::report::{json, table};
use heft::scan::detector::{BloatCategory, BloatEntry, Location};
use heft::scan::ScanResult;
use std::path::PathBuf;

fn test_entry(name: &str, category: BloatCategory, size: u64) -> BloatEntry {
    BloatEntry {
        category,
        name: name.to_string(),
        location: Location::FilesystemPath(PathBuf::from(format!("/tmp/{name}"))),
        size_bytes: size,
        reclaimable_bytes: size,
        last_modified: Some(1700000000),
        cleanup_hint: None,
        active: None,
        active_reason: None,
        staleness_score: Some(100.0),
    }
}

fn test_result(entries: Vec<BloatEntry>) -> ScanResult {
    ScanResult {
        entries,
        diagnostics: vec![],
        duration_ms: Some(100),
        detector_timings: vec![],
        peak_memory_bytes: None,
        detector_memory: vec![],
    }
}

#[test]
fn table_includes_all_entries() {
    let result = test_result(vec![
        test_entry("alpha", BloatCategory::ProjectArtifacts, 1000),
        test_entry("beta", BloatCategory::PackageCache, 2000),
        test_entry("gamma", BloatCategory::SystemCache, 3000),
    ]);

    let output = table::render(&result);
    assert!(
        output.contains("alpha"),
        "table should contain entry 'alpha'"
    );
    assert!(output.contains("beta"), "table should contain entry 'beta'");
    assert!(
        output.contains("gamma"),
        "table should contain entry 'gamma'"
    );
}

#[test]
fn table_shows_active_tag() {
    let mut entry = test_entry("my-project", BloatCategory::ProjectArtifacts, 5000);
    entry.active = Some(true);
    let result = test_result(vec![entry]);

    let output = table::render(&result);
    assert!(
        output.contains("[active]"),
        "active entry should have [active] tag"
    );
}

#[test]
fn table_handles_empty_result() {
    let result = test_result(vec![]);
    let output = table::render(&result);
    assert!(
        output.contains("No bloat"),
        "empty result should say no bloat detected"
    );
}

#[test]
fn table_large_file_category_renders() {
    let entry = test_entry("huge.iso", BloatCategory::LargeFile, 5_000_000_000);
    let result = test_result(vec![entry]);

    let output = table::render(&result);
    assert!(
        output.contains("LargeFile") || output.contains("Large"),
        "LargeFile category should render without panic: {output}"
    );
}

#[test]
fn json_output_is_valid() {
    let result = test_result(vec![test_entry(
        "node_modules",
        BloatCategory::ProjectArtifacts,
        500_000_000,
    )]);

    let output = json::render(&result);
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("should be valid JSON");

    let entries = parsed["entries"]
        .as_array()
        .expect("should have entries array");
    assert_eq!(entries.len(), 1);

    let entry = &entries[0];
    assert_eq!(entry["name"], "node_modules");
    assert_eq!(entry["size_bytes"], 500_000_000);
    assert_eq!(entry["category"], "ProjectArtifacts");
    assert!(
        entry["staleness_score"].is_number(),
        "staleness_score should be present"
    );
}

#[test]
fn json_includes_active_fields() {
    let mut entry = test_entry("target", BloatCategory::ProjectArtifacts, 1000);
    entry.active = Some(true);
    entry.active_reason = Some("git activity 2h ago".to_string());

    let result = test_result(vec![entry]);
    let output = json::render(&result);
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let e = &parsed["entries"][0];
    assert_eq!(e["active"], true);
    assert_eq!(e["active_reason"], "git activity 2h ago");
}

#[test]
fn json_large_file_category_serializes() {
    let entry = test_entry("backup.tar", BloatCategory::LargeFile, 10_000_000_000);
    let result = test_result(vec![entry]);

    let output = json::render(&result);
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["entries"][0]["category"], "LargeFile");
}
