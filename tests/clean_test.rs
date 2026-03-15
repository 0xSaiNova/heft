use std::fs;
use std::path::PathBuf;

use heft::clean::{self, CleanMode, CleanOptions};
use heft::scan::detector::{BloatCategory, BloatEntry, Location};
use heft::scan::ScanResult;

fn entry_at(path: PathBuf, active: Option<bool>, staleness: Option<f64>) -> BloatEntry {
    BloatEntry {
        category: BloatCategory::ProjectArtifacts,
        name: path.file_name().unwrap().to_string_lossy().to_string(),
        location: Location::FilesystemPath(path),
        size_bytes: 1000,
        reclaimable_bytes: 1000,
        last_modified: None,
        cleanup_hint: None,
        active,
        active_reason: if active == Some(true) {
            Some("test".to_string())
        } else {
            None
        },
        staleness_score: staleness,
        safety: None,
    }
}

fn scan_result(entries: Vec<BloatEntry>) -> ScanResult {
    ScanResult {
        entries,
        diagnostics: vec![],
        duration_ms: None,
        detector_timings: vec![],
        peak_memory_bytes: None,
        detector_memory: vec![],
    }
}

fn default_opts() -> CleanOptions {
    CleanOptions {
        category_filter: None,
        include_active: false,
        stale_only: false,
    }
}

#[test]
fn dry_run_does_not_delete() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-clean")
        .tempdir()
        .unwrap();
    let target = tmp.path().join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("file"), "data").unwrap();

    let result = scan_result(vec![entry_at(target.clone(), Some(false), Some(5.0))]);
    let cr = clean::run(&result, CleanMode::DryRun, default_opts());

    assert!(target.exists(), "dry-run must not delete directories");
    assert!(
        !cr.deleted.is_empty(),
        "dry-run should report what it would delete"
    );
}

#[test]
fn execute_mode_deletes() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-clean")
        .tempdir()
        .unwrap();
    let target = tmp.path().join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("file"), "data").unwrap();
    assert!(target.exists());

    let result = scan_result(vec![entry_at(target.clone(), Some(false), Some(5.0))]);
    let cr = clean::run(&result, CleanMode::Execute, default_opts());

    assert!(!target.exists(), "execute mode should delete the directory");
    assert_eq!(cr.bytes_freed, 1000);
}

#[test]
fn active_entries_skipped_when_not_included() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-clean")
        .tempdir()
        .unwrap();
    let active_dir = tmp.path().join("active");
    let stale_dir = tmp.path().join("stale");
    fs::create_dir(&active_dir).unwrap();
    fs::create_dir(&stale_dir).unwrap();

    let entries = vec![
        entry_at(active_dir.clone(), Some(true), Some(0.0)),
        entry_at(stale_dir.clone(), Some(false), Some(5.0)),
    ];
    let result = scan_result(entries);
    let opts = CleanOptions {
        include_active: false,
        ..default_opts()
    };
    let _cr = clean::run(&result, CleanMode::Execute, opts);

    assert!(active_dir.exists(), "active entry must be protected");
    assert!(!stale_dir.exists(), "stale entry should be deleted");
}

#[test]
fn active_entries_deleted_when_included() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-clean")
        .tempdir()
        .unwrap();
    let active_dir = tmp.path().join("active");
    fs::create_dir(&active_dir).unwrap();

    let result = scan_result(vec![entry_at(active_dir.clone(), Some(true), Some(0.0))]);
    let opts = CleanOptions {
        include_active: true,
        ..default_opts()
    };
    let _cr = clean::run(&result, CleanMode::Execute, opts);

    assert!(!active_dir.exists(), "include_active should allow deletion");
}

#[test]
fn stale_only_filters_fresh_entries() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-clean")
        .tempdir()
        .unwrap();
    let fresh_dir = tmp.path().join("fresh");
    let stale_dir = tmp.path().join("stale");
    let unknown_dir = tmp.path().join("unknown");
    fs::create_dir(&fresh_dir).unwrap();
    fs::create_dir(&stale_dir).unwrap();
    fs::create_dir(&unknown_dir).unwrap();

    let entries = vec![
        entry_at(fresh_dir.clone(), Some(false), Some(0.0)),
        entry_at(stale_dir.clone(), Some(false), Some(5.0)),
        entry_at(unknown_dir.clone(), Some(false), None),
    ];
    let result = scan_result(entries);
    let opts = CleanOptions {
        stale_only: true,
        ..default_opts()
    };
    let _cr = clean::run(&result, CleanMode::Execute, opts);

    assert!(fresh_dir.exists(), "fresh entry (score 0) must be skipped");
    assert!(
        !stale_dir.exists(),
        "stale entry (score 5) should be deleted"
    );
    assert!(
        unknown_dir.exists(),
        "unknown entry (no score) must be skipped"
    );
}

#[test]
fn category_filter_works() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-clean")
        .tempdir()
        .unwrap();
    let cache_dir = tmp.path().join("cache");
    let artifact_dir = tmp.path().join("artifact");
    fs::create_dir(&cache_dir).unwrap();
    fs::create_dir(&artifact_dir).unwrap();

    let mut cache_entry = entry_at(cache_dir.clone(), Some(false), Some(5.0));
    cache_entry.category = BloatCategory::PackageCache;
    let artifact_entry = entry_at(artifact_dir.clone(), Some(false), Some(5.0));

    let result = scan_result(vec![cache_entry, artifact_entry]);
    let opts = CleanOptions {
        category_filter: Some(vec![BloatCategory::PackageCache]),
        ..default_opts()
    };
    let _cr = clean::run(&result, CleanMode::Execute, opts);

    assert!(!cache_dir.exists(), "cache category should be cleaned");
    assert!(
        artifact_dir.exists(),
        "artifact category should be skipped by filter"
    );
}

#[test]
fn missing_path_does_not_panic() {
    let entry = entry_at(
        PathBuf::from("/tmp/heft-nonexistent-path-12345"),
        Some(false),
        Some(5.0),
    );
    let result = scan_result(vec![entry]);
    let cr = clean::run(&result, CleanMode::Execute, default_opts());

    assert!(
        !cr.errors.is_empty(),
        "should report error for missing path"
    );
}

#[test]
fn refuses_to_delete_outside_home() {
    // the clean engine validates paths are under $HOME or /tmp
    // /usr/share is neither, so it should be refused
    let entry = entry_at(
        PathBuf::from("/usr/share/fake-heft-test"),
        Some(false),
        Some(5.0),
    );
    let result = scan_result(vec![entry]);
    let cr = clean::run(&result, CleanMode::Execute, default_opts());

    assert!(
        cr.errors.iter().any(|e| e.contains("refusing")),
        "should refuse to delete path outside home: {:?}",
        cr.errors
    );
}
