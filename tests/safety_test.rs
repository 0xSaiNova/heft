use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use heft::safety::{self, SafetyInfo, SafetyTier};
use heft::scan::detector::{BloatCategory, BloatEntry, Location};

/// run a git command in a directory, with user config set so it works on CI
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@test.com")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@test.com")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn entry_with_category(category: BloatCategory, path: &str) -> BloatEntry {
    BloatEntry {
        category,
        name: "test".to_string(),
        location: Location::FilesystemPath(PathBuf::from(path)),
        size_bytes: 1000,
        reclaimable_bytes: 1000,
        last_modified: None,
        cleanup_hint: None,
        active: None,
        active_reason: None,
        staleness_score: Some(5.0),
        safety: None,
    }
}

#[test]
fn package_cache_is_disposable() {
    let entry = entry_with_category(BloatCategory::PackageCache, "/tmp/npm");
    let info = safety::classify_safety(&entry);
    assert_eq!(info.tier, SafetyTier::Disposable);
}

#[test]
fn system_cache_is_disposable() {
    let entry = entry_with_category(BloatCategory::SystemCache, "/tmp/cache");
    let info = safety::classify_safety(&entry);
    assert_eq!(info.tier, SafetyTier::Disposable);
}

#[test]
fn container_data_is_caution() {
    let entry = entry_with_category(BloatCategory::ContainerData, "/tmp/docker");
    let info = safety::classify_safety(&entry);
    assert_eq!(info.tier, SafetyTier::Caution);
}

#[test]
fn ide_data_is_caution() {
    let entry = entry_with_category(BloatCategory::IdeData, "/tmp/vscode");
    let info = safety::classify_safety(&entry);
    assert_eq!(info.tier, SafetyTier::Caution);
}

#[test]
fn other_is_caution() {
    let entry = entry_with_category(BloatCategory::Other, "/tmp/stuff");
    let info = safety::classify_safety(&entry);
    assert_eq!(info.tier, SafetyTier::Caution);
}

#[test]
fn large_file_is_user_data() {
    let entry = entry_with_category(BloatCategory::LargeFile, "/tmp/movie.iso");
    let info = safety::classify_safety(&entry);
    assert_eq!(info.tier, SafetyTier::UserData);
}

#[test]
fn clean_git_project_is_rebuildable() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-safety")
        .tempdir()
        .unwrap();
    let root = tmp.path();

    git(root, &["init"]);
    fs::write(root.join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join(".gitignore"), "target/\nnode_modules/\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "init"]);

    let target = root.join("target");
    fs::create_dir(&target).unwrap();

    let entry = entry_with_category(
        BloatCategory::ProjectArtifacts,
        &target.display().to_string(),
    );
    let info = safety::classify_safety(&entry);
    assert_eq!(info.tier, SafetyTier::Rebuildable);
    assert_eq!(info.git_dirty_files, Some(0));
}

#[test]
fn dirty_git_project_is_caution() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-safety")
        .tempdir()
        .unwrap();
    let root = tmp.path();

    git(root, &["init"]);
    fs::write(root.join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "init"]);

    // make it dirty with one uncommitted file
    fs::write(root.join("uncommitted.txt"), "dirty").unwrap();

    let target = root.join("target");
    fs::create_dir(&target).unwrap();

    let entry = entry_with_category(
        BloatCategory::ProjectArtifacts,
        &target.display().to_string(),
    );
    let info = safety::classify_safety(&entry);
    assert_eq!(info.tier, SafetyTier::Caution);
    assert_eq!(info.git_dirty_files, Some(1));
}

#[test]
fn no_git_project_is_caution() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-safety")
        .tempdir()
        .unwrap();
    let target = tmp.path().join("target");
    fs::create_dir(&target).unwrap();

    let entry = entry_with_category(
        BloatCategory::ProjectArtifacts,
        &target.display().to_string(),
    );
    let info = safety::classify_safety(&entry);
    assert_eq!(info.tier, SafetyTier::Caution);
    assert!(info.reason.contains("no git"));
}

#[test]
fn classify_all_deduplicates_git_checks() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-safety")
        .tempdir()
        .unwrap();
    let root = tmp.path();

    git(root, &["init"]);
    fs::write(root.join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join(".gitignore"), "target/\nnode_modules/\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "init"]);

    let target = root.join("target");
    let node_modules = root.join("node_modules");
    fs::create_dir(&target).unwrap();
    fs::create_dir(&node_modules).unwrap();

    let mut entries = vec![
        entry_with_category(
            BloatCategory::ProjectArtifacts,
            &target.display().to_string(),
        ),
        entry_with_category(
            BloatCategory::ProjectArtifacts,
            &node_modules.display().to_string(),
        ),
        entry_with_category(BloatCategory::PackageCache, "/tmp/npm"),
    ];

    safety::classify_all(&mut entries);

    assert_eq!(
        entries[0].safety.as_ref().unwrap().tier,
        SafetyTier::Rebuildable
    );
    assert_eq!(
        entries[1].safety.as_ref().unwrap().tier,
        SafetyTier::Rebuildable
    );
    assert_eq!(
        entries[2].safety.as_ref().unwrap().tier,
        SafetyTier::Disposable
    );
}

#[test]
fn should_preselect_respects_tiers() {
    let make = |tier: SafetyTier, staleness: f64, active: bool| -> BloatEntry {
        let mut e = entry_with_category(BloatCategory::PackageCache, "/tmp/test");
        e.staleness_score = Some(staleness);
        e.active = Some(active);
        e.safety = Some(SafetyInfo {
            tier,
            reason: "test".into(),
            git_dirty_files: None,
        });
        e
    };

    // disposable + stale = preselect
    assert!(safety::should_preselect(
        &make(SafetyTier::Disposable, 5.0, false),
        false
    ));
    // rebuildable + stale = preselect
    assert!(safety::should_preselect(
        &make(SafetyTier::Rebuildable, 5.0, false),
        false
    ));
    // caution + stale = no
    assert!(!safety::should_preselect(
        &make(SafetyTier::Caution, 5.0, false),
        false
    ));
    // user data + stale = no
    assert!(!safety::should_preselect(
        &make(SafetyTier::UserData, 5.0, false),
        false
    ));
    // fresh = no regardless of tier
    assert!(!safety::should_preselect(
        &make(SafetyTier::Disposable, 0.0, false),
        false
    ));
    // active = no unless include_active
    assert!(!safety::should_preselect(
        &make(SafetyTier::Disposable, 5.0, true),
        false
    ));
    assert!(safety::should_preselect(
        &make(SafetyTier::Disposable, 5.0, true),
        true
    ));
    // safety: None = no (unknown entries never preselected)
    let mut unknown = entry_with_category(BloatCategory::PackageCache, "/tmp/test");
    unknown.staleness_score = Some(5.0);
    unknown.active = Some(false);
    unknown.safety = None;
    assert!(!safety::should_preselect(&unknown, false));
}
