use std::fs;
use std::path::PathBuf;

use heft::big;
use heft::scan::detector::{BloatCategory, BloatEntry, Location};

#[test]
fn finds_files_above_threshold() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-big")
        .tempdir()
        .unwrap();

    // 500 KB (below 1 MB threshold)
    let small = tmp.path().join("small.bin");
    fs::write(&small, vec![0u8; 500_000]).unwrap();

    // 5 MB (above threshold)
    let medium = tmp.path().join("medium.bin");
    fs::write(&medium, vec![0u8; 5_000_000]).unwrap();

    // 10 MB (above threshold)
    let large = tmp.path().join("large.bin");
    fs::write(&large, vec![0u8; 10_000_000]).unwrap();

    let results = big::find_big_files(&[tmp.path().to_path_buf()], 1_000_000);
    let paths: Vec<&PathBuf> = results.iter().map(|r| &r.path).collect();

    assert!(!paths.contains(&&small), "500KB file should be excluded");
    assert!(paths.contains(&&medium), "5MB file should be found");
    assert!(paths.contains(&&large), "10MB file should be found");
}

#[test]
fn ignores_files_below_threshold() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-big")
        .tempdir()
        .unwrap();

    fs::write(tmp.path().join("tiny.txt"), "hello").unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 1000]).unwrap();

    let results = big::find_big_files(&[tmp.path().to_path_buf()], 1_000_000);
    assert!(results.is_empty(), "no files above threshold");
}

#[test]
fn does_not_follow_symlinks() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-big")
        .tempdir()
        .unwrap();

    // create a large file outside the scan root
    let external = tempfile::Builder::new()
        .prefix("heft-big-ext")
        .tempdir()
        .unwrap();
    let external_file = external.path().join("huge.bin");
    fs::write(&external_file, vec![0u8; 5_000_000]).unwrap();

    // symlink to it from inside the scan root
    #[cfg(unix)]
    std::os::unix::fs::symlink(&external_file, tmp.path().join("link.bin")).unwrap();

    let results = big::find_big_files(&[tmp.path().to_path_buf()], 1_000_000);
    let paths: Vec<String> = results
        .iter()
        .map(|r| r.path.display().to_string())
        .collect();

    assert!(
        !paths.iter().any(|p| p.contains("huge.bin")),
        "symlink target should not appear in results"
    );
}

#[test]
fn handles_permission_denied() {
    let tmp = tempfile::Builder::new()
        .prefix("heft-big")
        .tempdir()
        .unwrap();

    let locked = tmp.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::write(locked.join("secret.bin"), vec![0u8; 5_000_000]).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    }

    // should not panic
    let _results = big::find_big_files(&[tmp.path().to_path_buf()], 1_000_000);

    // restore permissions for cleanup
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&locked, fs::Permissions::from_mode(0o755));
    }
}

#[test]
fn dedup_removes_files_under_detector_paths() {
    let mut big_files = vec![
        big::BigFile {
            path: PathBuf::from("/home/user/project/target/debug/binary"),
            size_bytes: 50_000_000,
            mtime: None,
        },
        big::BigFile {
            path: PathBuf::from("/home/user/downloads/movie.mkv"),
            size_bytes: 2_000_000_000,
            mtime: None,
        },
    ];

    let detector_entries = vec![BloatEntry {
        category: BloatCategory::ProjectArtifacts,
        name: "project".to_string(),
        location: Location::FilesystemPath(PathBuf::from("/home/user/project/target")),
        size_bytes: 100_000_000,
        reclaimable_bytes: 100_000_000,
        last_modified: None,
        cleanup_hint: None,
        active: None,
        active_reason: None,
        staleness_score: None,
        safety: None,
    }];

    big::dedup_big_files(&mut big_files, &detector_entries);

    assert_eq!(big_files.len(), 1);
    assert!(big_files[0]
        .path
        .display()
        .to_string()
        .contains("movie.mkv"));
}

#[test]
fn dedup_handles_docker_locations() {
    let mut big_files = vec![big::BigFile {
        path: PathBuf::from("/home/user/file.bin"),
        size_bytes: 5_000_000,
        mtime: None,
    }];

    let detector_entries = vec![BloatEntry {
        category: BloatCategory::ContainerData,
        name: "docker images".to_string(),
        location: Location::DockerObject("sha256:abc123".to_string()),
        size_bytes: 1_000_000_000,
        reclaimable_bytes: 1_000_000_000,
        last_modified: None,
        cleanup_hint: None,
        active: None,
        active_reason: None,
        staleness_score: None,
        safety: None,
    }];

    // should not panic on DockerObject locations
    big::dedup_big_files(&mut big_files, &detector_entries);
    assert_eq!(
        big_files.len(),
        1,
        "docker entries should not affect filesystem dedup"
    );
}

#[test]
fn big_file_to_entry_produces_correct_fields() {
    let bf = big::BigFile {
        path: PathBuf::from("/home/user/downloads/backup.tar.gz"),
        size_bytes: 5_000_000_000,
        mtime: Some(1700000000),
    };

    let entry = big::big_file_to_entry(bf);

    assert!(entry.name.contains("backup.tar.gz"));
    assert_eq!(entry.category, BloatCategory::LargeFile);
    assert_eq!(entry.size_bytes, 5_000_000_000);
    assert_eq!(entry.reclaimable_bytes, 5_000_000_000);
    assert_eq!(entry.last_modified, Some(1700000000));
    assert!(entry.cleanup_hint.as_ref().unwrap().starts_with("rm "));
    match entry.location {
        Location::FilesystemPath(p) => {
            assert_eq!(
                p.display().to_string(),
                "/home/user/downloads/backup.tar.gz"
            );
        }
        _ => panic!("expected FilesystemPath location"),
    }
}
