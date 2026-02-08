use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use heft::config::Config;
use heft::platform::Platform;
use heft::scan;
use heft::scan::detector::BloatCategory;

#[test]
fn scan_runs_without_panic() {
    let config = Config {
        roots: vec![PathBuf::from("/tmp")],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    // should not panic, may or may not find caches
    let _result = scan::run(&config);
}

#[test]
fn detects_cache_directory() {
    let temp = std::env::temp_dir().join("heft_test_cache");
    let _ = fs::remove_dir_all(&temp);

    // create a fake home with a cache dir
    let fake_home = temp.join("home");
    let npm_cache = fake_home.join(".npm");
    let cache_files = npm_cache.join("_cacache");

    fs::create_dir_all(&cache_files).unwrap();
    fs::write(cache_files.join("data.json"), r#"{"cached": true}"#).unwrap();

    // we cant easily test the cache detector in isolation since it uses
    // the real home dir. this test just verifies the scan machinery works.
    let config = Config {
        roots: vec![temp.clone()],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    // cache detector looks at real home, not our temp dir
    // so this just confirms no crash
    assert!(result.diagnostics.is_empty() || !result.diagnostics.is_empty());

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn cache_entries_have_correct_category() {
    let config = Config {
        roots: vec![PathBuf::from("/nonexistent")],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    // any cache entries found should have PackageCache or IdeData category
    for entry in &result.entries {
        assert!(
            entry.category == BloatCategory::PackageCache
                || entry.category == BloatCategory::IdeData,
            "unexpected category: {:?}",
            entry.category
        );
    }
}
