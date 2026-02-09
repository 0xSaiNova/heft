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

        if config.platform == Platform::Unknown {
            diagnostics.push("unknown platform detected, falling back to Unix-like cache paths".to_string());
        }

        let home = match platform::home_dir() {
            Some(h) => h,
            None => {
                return DetectorResult::with_diagnostic("could not determine home directory".into());
            }
        };

        let (caches, cache_diagnostics) = get_cache_locations(&home, config.platform);
        diagnostics.extend(cache_diagnostics);

        for cache in caches {
            if !cache.path.exists() {
                continue;
            }

            match calculate_dir_size(&cache.path) {
                Ok((size, warnings)) if size > 0 => {
                    entries.push(BloatEntry {
                        category: cache.category,
                        name: cache.name.to_string(),
                        location: Location::FilesystemPath(cache.path.clone()),
                        size_bytes: size,
                        reclaimable_bytes: size,
                        last_modified: None,
                        cleanup_hint: Some(cache.cleanup_hint.to_string()),
                    });

                    for warning in warnings {
                        diagnostics.push(format!("{warning} (size may be underestimated)"));
                    }
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

fn get_cache_locations(home: &Path, platform: Platform) -> (Vec<CacheLocation>, Vec<String>) {
    let mut locations = Vec::new();
    let mut diagnostics = Vec::new();

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
        Platform::Linux | Platform::Windows | Platform::Unknown => home.join(".cache/yarn"),
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
        Platform::Linux | Platform::Windows | Platform::Unknown => home.join(".cache/pip"),
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
    match get_homebrew_cache() {
        Ok(Some(brew_cache)) => {
            locations.push(CacheLocation {
                name: "homebrew cache",
                path: brew_cache,
                category: BloatCategory::PackageCache,
                cleanup_hint: "brew cleanup",
            });
        }
        Ok(None) => {
            // brew not installed, this is normal
        }
        Err(e) => {
            diagnostics.push(format!("homebrew cache detection failed: {e}"));
        }
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
        Platform::Linux | Platform::Windows | Platform::Unknown => home.join(".config/Code"),
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

    (locations, diagnostics)
}

fn get_homebrew_cache() -> Result<Option<PathBuf>, String> {
    use std::time::Duration;
    use std::process::Stdio;
    use std::io::Read;

    let mut child = match Command::new("brew")
        .arg("--cache")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(e) => {
            return Err(format!("failed to spawn brew command: {e}"));
        }
    };

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let mut stderr = String::new();
                    if let Some(mut stderr_pipe) = child.stderr.take() {
                        let _ = stderr_pipe.read_to_string(&mut stderr);
                    }
                    return Err(format!("brew --cache failed with status {}: {}", status.code().unwrap_or(-1), stderr.trim()));
                }

                let mut output = String::new();
                let mut stdout = child.stdout.take()
                    .ok_or_else(|| "failed to capture brew stdout".to_string())?;

                if let Err(e) = stdout.read_to_string(&mut output) {
                    return Err(format!("failed to read brew output: {e}"));
                }

                let path_str = output.trim();
                if path_str.is_empty() {
                    return Err("brew returned empty output".to_string());
                }

                let path = PathBuf::from(path_str);
                if path.exists() {
                    return Ok(Some(path));
                } else {
                    return Err(format!("brew returned path {} but it doesn't exist", path.display()));
                }
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err("brew --cache timed out after 5 seconds".to_string());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!("failed to wait for brew process: {e}"));
            }
        }
    }
}

fn calculate_dir_size(path: &Path) -> Result<(u64, Vec<String>), std::io::Error> {
    let mut total = 0u64;
    let mut warnings = Vec::new();
    let mut overflowed = false;

    for entry in WalkDir::new(path).follow_links(false).into_iter() {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        let file_size = metadata.len();
                        match total.checked_add(file_size) {
                            Some(new_total) => total = new_total,
                            None => {
                                if !overflowed {
                                    warnings.push("directory size exceeds u64::MAX, size capped at maximum value".to_string());
                                    overflowed = true;
                                }
                                total = u64::MAX;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if e.io_error().map(|io_err| io_err.kind() == std::io::ErrorKind::PermissionDenied).unwrap_or(false) {
                    warnings.push(format!("permission denied: {}", e.path().map(|p| p.display().to_string()).unwrap_or_else(|| "unknown path".to_string())));
                }
            }
        }
    }

    Ok((total, warnings))
}
