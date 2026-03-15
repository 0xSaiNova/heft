//! Large file discovery for the default flow.

use std::path::{Path, PathBuf};

use crate::scan::detector::{BloatCategory, BloatEntry, Location};

pub struct BigFile {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub mtime: Option<i64>,
}

const SKIP_PATHS: &[&str] = &["/proc", "/sys", "/dev", "/run"];

pub fn find_big_files(roots: &[PathBuf], min_bytes: u64) -> Vec<BigFile> {
    let mut results = Vec::new();
    for root in roots {
        let root_dev = get_device_id(root);
        for entry in jwalk::WalkDir::new(root)
            .skip_hidden(false)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if SKIP_PATHS.iter().any(|p| path.starts_with(p)) {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if let Some(dev) = root_dev {
                    if metadata.dev() != dev {
                        continue;
                    }
                }
            }
            if metadata.is_file() && metadata.len() >= min_bytes {
                results.push(BigFile {
                    path: path.to_path_buf(),
                    size_bytes: metadata.len(),
                    mtime: metadata.modified().ok().and_then(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|d| d.as_secs() as i64)
                    }),
                });
            }
        }
    }
    results
}

pub fn dedup_big_files(big_files: &mut Vec<BigFile>, detector_entries: &[BloatEntry]) {
    let detector_paths: Vec<&Path> = detector_entries
        .iter()
        .filter_map(|e| match &e.location {
            Location::FilesystemPath(p) => Some(p.as_path()),
            _ => None,
        })
        .collect();
    big_files.retain(|f| !detector_paths.iter().any(|dp| f.path.starts_with(dp)));
}

pub fn big_file_to_entry(bf: BigFile) -> BloatEntry {
    let file_name = bf
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    BloatEntry {
        name: format!("Large file: {file_name}"),
        location: Location::FilesystemPath(bf.path.clone()),
        size_bytes: bf.size_bytes,
        reclaimable_bytes: bf.size_bytes,
        category: BloatCategory::LargeFile,
        last_modified: bf.mtime,
        cleanup_hint: Some(format!("rm \"{}\"", bf.path.display())),
        active: None,
        active_reason: None,
        staleness_score: None,
    }
}

#[cfg(unix)]
fn get_device_id(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|m| m.dev())
}

#[cfg(not(unix))]
fn get_device_id(_path: &Path) -> Option<u64> {
    None
}
