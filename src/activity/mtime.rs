//! Source file modification time signal for activity detection.
//!
//! Samples source files breadth-first from a project root, skipping artifact
//! directories. Bounded by both a file sample limit and a max directory depth
//! to avoid deep traversal into large monorepos.

use std::collections::VecDeque;
use std::path::Path;
use std::time::SystemTime;

use crate::scan::detector::{ARTIFACT_DIR_NAMES, SOURCE_EXTENSIONS};

/// Priority directories to check first (highest signal for developer activity).
const PRIORITY_DIRS: &[&str] = &["src", "lib", "app", "pkg"];

/// Max directory depth to descend. Matches the original max_depth(3) constraint
/// from projects.rs to avoid scanning deep into monorepos.
const MAX_DEPTH: usize = 4;

/// Sample up to `limit` source files breadth-first from root,
/// skipping artifact directories. Bounded to MAX_DEPTH levels deep.
pub fn latest_source_mtime(root: &Path, limit: usize) -> Option<SystemTime> {
    let mut latest: Option<SystemTime> = None;
    let mut sampled = 0usize;
    let mut queue: VecDeque<(std::path::PathBuf, usize)> = VecDeque::new();

    // seed queue: priority directories first, then everything else
    let mut priority = Vec::new();
    let mut rest = Vec::new();

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                if ARTIFACT_DIR_NAMES.contains(&name_str.as_ref()) {
                    continue;
                }
                if name_str.starts_with('.') {
                    continue;
                }
                if PRIORITY_DIRS.contains(&name_str.as_ref()) {
                    priority.push(entry.path());
                } else {
                    rest.push(entry.path());
                }
            } else {
                // check top-level source files immediately
                if let Some(mtime) = check_source_file(&entry.path()) {
                    latest = Some(latest.map_or(mtime, |l: SystemTime| l.max(mtime)));
                    sampled += 1;
                    if sampled >= limit {
                        return latest;
                    }
                }
            }
        }
    }

    // enqueue priority dirs first for breadth-first traversal (depth 1)
    for dir in priority {
        queue.push_back((dir, 1));
    }
    for dir in rest {
        queue.push_back((dir, 1));
    }

    // breadth-first walk with depth limit
    while let Some((dir, depth)) = queue.pop_front() {
        if sampled >= limit {
            break;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if ft.is_dir() {
                if depth < MAX_DEPTH {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if !ARTIFACT_DIR_NAMES.contains(&name_str.as_ref())
                        && !name_str.starts_with('.')
                    {
                        queue.push_back((entry.path(), depth + 1));
                    }
                }
            } else if ft.is_file() {
                if let Some(mtime) = check_source_file(&entry.path()) {
                    latest = Some(latest.map_or(mtime, |l: SystemTime| l.max(mtime)));
                    sampled += 1;
                    if sampled >= limit {
                        return latest;
                    }
                }
            }
        }
    }

    latest
}

fn check_source_file(path: &Path) -> Option<SystemTime> {
    let ext = path.extension()?.to_str()?;
    if !SOURCE_EXTENSIONS.contains(&ext) {
        return None;
    }
    std::fs::metadata(path).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_source_files_in_src_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("main.rs"), "fn main() {}").unwrap();

        let result = latest_source_mtime(tmp.path(), 200);
        assert!(result.is_some());
    }

    #[test]
    fn skips_artifact_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target").join("debug");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("lib.rs"), "// artifact").unwrap();

        // no source files outside artifacts
        let result = latest_source_mtime(tmp.path(), 200);
        assert!(result.is_none());
    }

    #[test]
    fn respects_sample_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();

        for i in 0..10 {
            fs::write(src.join(format!("file{i}.rs")), "// code").unwrap();
        }

        // with limit of 3, should still return a result (just checks fewer files)
        let result = latest_source_mtime(tmp.path(), 3);
        assert!(result.is_some());
    }

    #[test]
    fn empty_dir_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(latest_source_mtime(tmp.path(), 200).is_none());
    }

    #[test]
    fn respects_depth_limit() {
        let tmp = tempfile::tempdir().unwrap();
        // create a source file deeper than MAX_DEPTH
        let deep = tmp.path().join("a").join("b").join("c").join("d").join("e");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("deep.rs"), "// too deep").unwrap();

        // should not find it since MAX_DEPTH is 4
        let result = latest_source_mtime(tmp.path(), 200);
        assert!(result.is_none());
    }
}
