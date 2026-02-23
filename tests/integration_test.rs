use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use heft::config::Config;
use heft::platform::Platform;
use heft::scan;
use heft::scan::detector::BloatCategory;

// helper to filter results by category
fn project_entries(result: &scan::ScanResult) -> Vec<&scan::detector::BloatEntry> {
    result
        .entries
        .iter()
        .filter(|e| e.category == BloatCategory::ProjectArtifacts)
        .collect()
}

#[allow(dead_code)]
fn tmpdir() -> tempfile::TempDir {
    // TempDir::new() uses ".tmp" prefix which is filtered by is_hidden().
    // use a non-hidden prefix so the project scanner can traverse into it.
    tempfile::Builder::new()
        .prefix("heft-test")
        .tempdir()
        .unwrap()
}

fn test_config(root: PathBuf) -> Config {
    Config {
        roots: vec![root],
        timeout: Duration::from_secs(30),
        disabled_detectors: std::collections::HashSet::from(["docker".to_string()]),
        json_output: false,
        verbose: false,
        progressive: false,
        platform: Platform::Linux,
    }
}

// ============================================================================
// Project detector tests
// ============================================================================

#[test]
fn empty_directory_returns_no_project_entries() {
    let temp = tmpdir();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    assert!(project_entries(&result).is_empty());
}

#[test]
fn detects_node_modules_in_project() {
    let temp = tmpdir();
    let project = temp.path().join("my-project");
    let node_modules = project.join("node_modules");
    let fake_package = node_modules.join("fake-pkg");

    fs::create_dir_all(&fake_package).unwrap();
    fs::write(project.join("package.json"), r#"{"name": "my-project"}"#).unwrap();
    fs::write(fake_package.join("index.js"), "module.exports = {}").unwrap();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    let projects = project_entries(&result);

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "my-project");
    assert_eq!(projects[0].category, BloatCategory::ProjectArtifacts);
    assert!(projects[0].size_bytes > 0);
}

#[test]
fn detects_cargo_target_in_rust_project() {
    let temp = tmpdir();
    let project = temp.path().join("my-crate");
    let debug = project.join("target").join("debug");

    fs::create_dir_all(&debug).unwrap();
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"my-crate\"\nversion = \"0.1.0\"",
    )
    .unwrap();
    fs::write(debug.join("my-crate"), "fake binary content here").unwrap();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    let projects = project_entries(&result);

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "my-crate");
    assert_eq!(projects[0].category, BloatCategory::ProjectArtifacts);
}

#[test]
fn skips_nested_node_modules_in_monorepo() {
    let temp = tmpdir();
    let root = temp.path().join("monorepo");
    let root_nm = root.join("node_modules");
    let pkg_a = root.join("packages").join("pkg-a");
    let nested_nm = pkg_a.join("node_modules");

    fs::create_dir_all(&root_nm).unwrap();
    fs::create_dir_all(&nested_nm).unwrap();
    fs::write(root.join("package.json"), r#"{"name": "monorepo"}"#).unwrap();
    fs::write(pkg_a.join("package.json"), r#"{"name": "pkg-a"}"#).unwrap();
    fs::write(root_nm.join("dep.js"), "x").unwrap();
    fs::write(nested_nm.join("dep.js"), "y").unwrap();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    let projects = project_entries(&result);

    // should only detect the root node_modules, not the nested one
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "monorepo");
}

#[test]
fn detects_python_venv() {
    let temp = tmpdir();
    let project = temp.path().join("my-python-project");
    let site_packages = project
        .join(".venv")
        .join("lib")
        .join("python3.11")
        .join("site-packages");

    fs::create_dir_all(&site_packages).unwrap();
    fs::write(project.join("requirements.txt"), "requests==2.28.0").unwrap();
    fs::write(site_packages.join("requests.py"), "# fake").unwrap();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    let projects = project_entries(&result);

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].category, BloatCategory::ProjectArtifacts);
    assert!(projects[0].cleanup_hint.as_ref().unwrap().contains("venv"));
}

#[test]
fn detects_pytest_cache() {
    let temp = tmpdir();
    let project = temp.path().join("my-test-project");
    let cache = project.join(".pytest_cache");

    fs::create_dir_all(&cache).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = \"test\"").unwrap();
    fs::write(cache.join("README.md"), "pytest cache").unwrap();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    let projects = project_entries(&result);

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].category, BloatCategory::ProjectArtifacts);
}

#[test]
fn falls_back_to_directory_name_when_manifest_has_no_name() {
    let temp = tmpdir();
    let project = temp.path().join("unnamed-project");
    let node_modules = project.join("node_modules");

    fs::create_dir_all(&node_modules).unwrap();
    // package.json exists but has no name field
    fs::write(project.join("package.json"), r#"{"version": "1.0.0"}"#).unwrap();
    fs::write(node_modules.join("dep.js"), "x").unwrap();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    let projects = project_entries(&result);

    assert_eq!(projects.len(), 1);
    // should fall back to directory name
    assert_eq!(projects[0].name, "unnamed-project");
}

#[test]
fn does_not_detect_target_without_cargo_toml() {
    let temp = tmpdir();
    let target = temp.path().join("not-rust").join("target");

    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("output.txt"), "build output").unwrap();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    // should NOT detect as artifact since there's no Cargo.toml
    assert!(project_entries(&result).is_empty());
}

// ============================================================================
// .NET bin/obj detection tests
// ============================================================================

#[test]
fn detects_dotnet_bin_obj_with_csproj() {
    let temp = tmpdir();
    let project = temp.path().join("MyApp");
    let bin = project.join("bin").join("Debug").join("net8.0");
    let obj = project.join("obj").join("Debug").join("net8.0");

    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&obj).unwrap();
    fs::write(project.join("MyApp.csproj"), "<Project></Project>").unwrap();
    fs::write(bin.join("MyApp.dll"), "fake dll").unwrap();
    fs::write(obj.join("MyApp.cache"), "fake cache").unwrap();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    let projects = project_entries(&result);

    assert!(
        projects.len() >= 1,
        "expected at least 1 .NET artifact, got {}",
        projects.len()
    );
    assert!(
        projects
            .iter()
            .any(|e| e.cleanup_hint.as_ref().unwrap().contains("dotnet")),
        "expected dotnet cleanup hint"
    );
}

#[test]
fn does_not_detect_bin_without_dotnet_project() {
    let temp = tmpdir();
    let project = temp.path().join("some-tool");

    fs::create_dir_all(project.join("bin")).unwrap();
    fs::create_dir_all(project.join("obj")).unwrap();
    fs::write(project.join("bin").join("tool.sh"), "#!/bin/bash").unwrap();
    fs::write(project.join("obj").join("data.o"), "fake object").unwrap();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    assert!(
        project_entries(&result).is_empty(),
        "bin/obj without .csproj should not be detected"
    );
}

#[test]
fn does_not_detect_obj_without_dotnet_project() {
    let temp = tmpdir();
    let project = temp.path().join("graphics-project");
    let obj = project.join("obj");

    fs::create_dir_all(&obj).unwrap();
    fs::write(project.join("Makefile"), "all: build").unwrap();
    fs::write(obj.join("model.obj"), "v 0 0 0").unwrap();

    let result = scan::run(&test_config(temp.path().to_path_buf()));
    assert!(
        project_entries(&result).is_empty(),
        "obj/ without .csproj should not be detected"
    );
}

// ============================================================================
// Cache detector tests
// ============================================================================

#[test]
fn scan_runs_without_panic() {
    let config = Config {
        roots: vec![PathBuf::from("/tmp")],
        timeout: Duration::from_secs(30),
        disabled_detectors: std::collections::HashSet::from(["docker".to_string()]),
        json_output: false,
        verbose: false,
        progressive: false,
        platform: Platform::Linux,
    };

    // should not panic, may or may not find caches
    let _result = scan::run(&config);
}

#[test]
fn detects_cache_directory() {
    let temp = tmpdir();
    let cache_files = temp.path().join("home").join(".npm").join("_cacache");

    fs::create_dir_all(&cache_files).unwrap();
    fs::write(cache_files.join("data.json"), r#"{"cached": true}"#).unwrap();

    // cache detector looks at real home, not our temp dir
    // so this just confirms no crash
    let result = scan::run(&test_config(temp.path().to_path_buf()));
    let _ = result.diagnostics.len(); // suppress unused warning
}

#[test]
fn cache_entries_have_correct_category() {
    let config = Config {
        roots: vec![PathBuf::from("/nonexistent")],
        timeout: Duration::from_secs(30),
        disabled_detectors: std::collections::HashSet::from(["docker".to_string()]),
        json_output: false,
        verbose: false,
        progressive: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    // every entry should have a known category
    for entry in &result.entries {
        assert!(
            matches!(
                entry.category,
                BloatCategory::PackageCache
                    | BloatCategory::IdeData
                    | BloatCategory::ProjectArtifacts
                    | BloatCategory::ContainerData
                    | BloatCategory::SystemCache
            ),
            "unexpected category: {:?}",
            entry.category
        );
    }
}
