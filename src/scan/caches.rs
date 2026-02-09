//! Detects package manager and toolchain caches.

use std::path::{Path, PathBuf};
use std::process::Command;

use walkdir::WalkDir;

use crate::config::Config;
use crate::platform::{self, Platform};
use super::detector::{BloatCategory, BloatEntry, Detector, DetectorResult, Location};

pub struct CacheDetector;

impl Detector for CacheDetector {
    fn name(&self) -> &'static str {
        "caches"
    }

    fn available(&self, _config: &Config) -> bool {
        true
    }

    fn scan(&self, config: &Config) -> DetectorResult {
        let mut entries = Vec::new();
        let mut diagnostics = Vec::new();

        let home = match platform::home_dir() {
            Some(h) => h,
            None => {
                return DetectorResult::with_diagnostic("could not determine home directory".into());
            }
        };

        let caches = get_cache_locations(&home, config.platform);

        for cache in caches {
            if !cache.path.exists() {
                continue;
            }

            match calculate_dir_size(&cache.path) {
                Ok(size) if size > 0 => {
                    entries.push(BloatEntry {
                        category: cache.category,
                        name: cache.name.to_string(),
                        location: Location::FilesystemPath(cache.path.clone()),
                        size_bytes: size,
                        reclaimable_bytes: size,
                        last_modified: None,
                        cleanup_hint: Some(cache.cleanup_hint.to_string()),
                    });
                }
                Ok(_) => {}
                Err(e) => {
                    diagnostics.push(format!("failed to scan {}: {}", cache.path.display(), e));
                }
            }
        }

        DetectorResult { entries, diagnostics }
    }
}

struct CacheLocation {
    name: &'static str,
    path: PathBuf,
    category: BloatCategory,
    cleanup_hint: &'static str,
}

fn get_cache_locations(home: &Path, platform: Platform) -> Vec<CacheLocation> {
    let mut locations = Vec::new();

    // npm cache
    locations.push(CacheLocation {
        name: "npm cache",
        path: home.join(".npm"),
        category: BloatCategory::PackageCache,
        cleanup_hint: "npm cache clean --force",
    });

    // yarn cache
    let yarn_path = match platform {
        Platform::MacOS => home.join("Library/Caches/Yarn"),
        _ => home.join(".cache/yarn"),
    };
    locations.push(CacheLocation {
        name: "yarn cache",
        path: yarn_path,
        category: BloatCategory::PackageCache,
        cleanup_hint: "yarn cache clean",
    });

    // pnpm store
    locations.push(CacheLocation {
        name: "pnpm store",
        path: home.join(".local/share/pnpm/store"),
        category: BloatCategory::PackageCache,
        cleanup_hint: "pnpm store prune",
    });

    // pip cache
    let pip_path = match platform {
        Platform::MacOS => home.join("Library/Caches/pip"),
        _ => home.join(".cache/pip"),
    };
    locations.push(CacheLocation {
        name: "pip cache",
        path: pip_path,
        category: BloatCategory::PackageCache,
        cleanup_hint: "pip cache purge",
    });

    // cargo registry and git checkouts
    locations.push(CacheLocation {
        name: "cargo registry",
        path: home.join(".cargo/registry"),
        category: BloatCategory::PackageCache,
        cleanup_hint: "cargo cache --autoclean (requires cargo-cache)",
    });
    locations.push(CacheLocation {
        name: "cargo git",
        path: home.join(".cargo/git"),
        category: BloatCategory::PackageCache,
        cleanup_hint: "cargo cache --autoclean (requires cargo-cache)",
    });

    // homebrew cache (macOS and Linux)
    if let Some(brew_cache) = get_homebrew_cache() {
        locations.push(CacheLocation {
            name: "homebrew cache",
            path: brew_cache,
            category: BloatCategory::PackageCache,
            cleanup_hint: "brew cleanup",
        });
    }

    // go module cache
    locations.push(CacheLocation {
        name: "go module cache",
        path: home.join("go/pkg/mod"),
        category: BloatCategory::PackageCache,
        cleanup_hint: "go clean -modcache",
    });

    // VS Code extensions and cache
    let vscode_path = match platform {
        Platform::MacOS => home.join("Library/Application Support/Code"),
        _ => home.join(".config/Code"),
    };
    locations.push(CacheLocation {
        name: "vscode data",
        path: vscode_path,
        category: BloatCategory::IdeData,
        cleanup_hint: "clear from within vscode or delete unused extensions",
    });

    // gradle cache
    locations.push(CacheLocation {
        name: "gradle cache",
        path: home.join(".gradle/caches"),
        category: BloatCategory::PackageCache,
        cleanup_hint: "rm -rf ~/.gradle/caches",
    });

    // maven cache
    locations.push(CacheLocation {
        name: "maven cache",
        path: home.join(".m2/repository"),
        category: BloatCategory::PackageCache,
        cleanup_hint: "mvn dependency:purge-local-repository",
    });

    locations
}

fn get_homebrew_cache() -> Option<PathBuf> {
    use std::time::Duration;
    use std::process::Stdio;
    use std::io::Read;

    let mut child = Command::new("brew")
        .arg("--cache")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    // wait with a 5 second timeout
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }

                let mut output = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    stdout.read_to_string(&mut output).ok()?;
                }
                let path = PathBuf::from(output.trim());

                return if path.exists() { Some(path) } else { None };
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    // timeout reached, kill the process
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return None,
        }
    }
}

fn calculate_dir_size(path: &Path) -> Result<u64, std::io::Error> {
    let mut total = 0u64;

    for entry in WalkDir::new(path).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total += metadata.len();
            }
        }
    }

    Ok(total)
}
