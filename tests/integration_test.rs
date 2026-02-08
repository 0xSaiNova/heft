use std::fs;
use std::time::Duration;

use heft::config::Config;
use heft::platform::Platform;
use heft::scan;
use heft::scan::detector::BloatCategory;

#[test]
fn empty_directory_returns_no_entries() {
    let temp = std::env::temp_dir().join("heft_test_empty");
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();

    let config = Config {
        roots: vec![temp.clone()],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);
    assert!(result.entries.is_empty());

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn detects_node_modules_in_project() {
    let temp = std::env::temp_dir().join("heft_test_node");
    let _ = fs::remove_dir_all(&temp);

    let project = temp.join("my-project");
    let node_modules = project.join("node_modules");
    let fake_package = node_modules.join("fake-pkg");

    fs::create_dir_all(&fake_package).unwrap();
    fs::write(project.join("package.json"), r#"{"name": "my-project"}"#).unwrap();
    fs::write(fake_package.join("index.js"), "module.exports = {}").unwrap();

    let config = Config {
        roots: vec![temp.clone()],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].name, "my-project");
    assert_eq!(result.entries[0].category, BloatCategory::ProjectArtifacts);
    assert!(result.entries[0].size_bytes > 0);

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn detects_cargo_target_in_rust_project() {
    let temp = std::env::temp_dir().join("heft_test_rust");
    let _ = fs::remove_dir_all(&temp);

    let project = temp.join("my-crate");
    let target = project.join("target");
    let debug = target.join("debug");

    fs::create_dir_all(&debug).unwrap();
    fs::write(project.join("Cargo.toml"), "[package]\nname = \"my-crate\"\nversion = \"0.1.0\"").unwrap();
    fs::write(debug.join("my-crate"), "fake binary content here").unwrap();

    let config = Config {
        roots: vec![temp.clone()],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].name, "my-crate");
    assert_eq!(result.entries[0].category, BloatCategory::ProjectArtifacts);

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn skips_nested_node_modules_in_monorepo() {
    let temp = std::env::temp_dir().join("heft_test_monorepo");
    let _ = fs::remove_dir_all(&temp);

    let root = temp.join("monorepo");
    let root_nm = root.join("node_modules");
    let pkg_a = root.join("packages").join("pkg-a");
    let nested_nm = pkg_a.join("node_modules");

    fs::create_dir_all(&root_nm).unwrap();
    fs::create_dir_all(&nested_nm).unwrap();
    fs::write(root.join("package.json"), r#"{"name": "monorepo"}"#).unwrap();
    fs::write(pkg_a.join("package.json"), r#"{"name": "pkg-a"}"#).unwrap();
    fs::write(root_nm.join("dep.js"), "x").unwrap();
    fs::write(nested_nm.join("dep.js"), "y").unwrap();

    let config = Config {
        roots: vec![temp.clone()],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    // should only detect the root node_modules, not the nested one
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].name, "monorepo");

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn detects_python_venv() {
    let temp = std::env::temp_dir().join("heft_test_python");
    let _ = fs::remove_dir_all(&temp);

    let project = temp.join("my-python-project");
    let venv = project.join(".venv");
    let site_packages = venv.join("lib").join("python3.11").join("site-packages");

    fs::create_dir_all(&site_packages).unwrap();
    fs::write(project.join("requirements.txt"), "requests==2.28.0").unwrap();
    fs::write(site_packages.join("requests.py"), "# fake").unwrap();

    let config = Config {
        roots: vec![temp.clone()],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].category, BloatCategory::ProjectArtifacts);
    assert!(result.entries[0].cleanup_hint.as_ref().unwrap().contains("venv"));

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn detects_pytest_cache() {
    let temp = std::env::temp_dir().join("heft_test_pytest");
    let _ = fs::remove_dir_all(&temp);

    let project = temp.join("my-test-project");
    let cache = project.join(".pytest_cache");

    fs::create_dir_all(&cache).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = \"test\"").unwrap();
    fs::write(cache.join("v").join("cache").join("data"), "cached").ok();
    fs::write(cache.join("README.md"), "pytest cache").unwrap();

    let config = Config {
        roots: vec![temp.clone()],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].category, BloatCategory::ProjectArtifacts);

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn falls_back_to_directory_name_when_manifest_has_no_name() {
    let temp = std::env::temp_dir().join("heft_test_fallback");
    let _ = fs::remove_dir_all(&temp);

    let project = temp.join("unnamed-project");
    let node_modules = project.join("node_modules");

    fs::create_dir_all(&node_modules).unwrap();
    // package.json exists but has no name field
    fs::write(project.join("package.json"), r#"{"version": "1.0.0"}"#).unwrap();
    fs::write(node_modules.join("dep.js"), "x").unwrap();

    let config = Config {
        roots: vec![temp.clone()],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    assert_eq!(result.entries.len(), 1);
    // should fall back to directory name
    assert_eq!(result.entries[0].name, "unnamed-project");

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn does_not_detect_target_without_cargo_toml() {
    let temp = std::env::temp_dir().join("heft_test_no_cargo");
    let _ = fs::remove_dir_all(&temp);

    let project = temp.join("not-rust");
    let target = project.join("target");

    fs::create_dir_all(&target).unwrap();
    // no Cargo.toml - target could be a different kind of directory
    fs::write(target.join("output.txt"), "build output").unwrap();

    let config = Config {
        roots: vec![temp.clone()],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    // should NOT detect as artifact since there's no Cargo.toml
    assert!(result.entries.is_empty());

    let _ = fs::remove_dir_all(&temp);
}
