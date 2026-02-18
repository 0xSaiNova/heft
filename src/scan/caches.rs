//! Detects package manager and toolchain caches.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::detector::{BloatCategory, BloatEntry, Detector, DetectorResult, Location};
use crate::config::Config;
use crate::platform::{self, Platform};

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
            diagnostics.push(
                "unknown platform detected, falling back to Unix-like cache paths".to_string(),
            );
        }

        let home = match platform::home_dir() {
            Some(h) => h,
            None => {
                return DetectorResult::with_diagnostic(
                    "could not determine home directory".into(),
                );
            }
        };

        let (caches, cache_diagnostics) =
            get_cache_locations(&home, config.platform, config.timeout);
        diagnostics.extend(cache_diagnostics);

        for cache in caches {
            if !cache.path.exists() {
                continue;
            }

            match super::calculate_dir_size(&cache.path) {
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

        DetectorResult {
            entries,
            diagnostics,
        }
    }
}

struct CacheLocation {
    name: &'static str,
    path: PathBuf,
    category: BloatCategory,
    cleanup_hint: &'static str,
}

fn get_cache_locations(
    home: &Path,
    platform: Platform,
    timeout: Duration,
) -> (Vec<CacheLocation>, Vec<String>) {
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
        Platform::Windows => home
            .join("AppData")
            .join("Local")
            .join("Yarn")
            .join("Cache"),
        Platform::Linux | Platform::Unknown => home.join(".cache/yarn"),
    };
    locations.push(CacheLocation {
        name: "yarn cache",
        path: yarn_path,
        category: BloatCategory::PackageCache,
        cleanup_hint: "yarn cache clean",
    });

    // pnpm store
    let pnpm_path = match platform {
        Platform::MacOS => home.join("Library/pnpm/store"),
        Platform::Windows => home
            .join("AppData")
            .join("Local")
            .join("pnpm")
            .join("store"),
        Platform::Linux | Platform::Unknown => home.join(".local/share/pnpm/store"),
    };
    locations.push(CacheLocation {
        name: "pnpm store",
        path: pnpm_path,
        category: BloatCategory::PackageCache,
        cleanup_hint: "pnpm store prune",
    });

    // pip cache
    let pip_path = match platform {
        Platform::MacOS => home.join("Library/Caches/pip"),
        Platform::Windows => home.join("AppData").join("Local").join("pip").join("Cache"),
        Platform::Linux | Platform::Unknown => home.join(".cache/pip"),
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
    match get_homebrew_cache(timeout) {
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
        Platform::Windows => home.join("AppData").join("Roaming").join("Code"),
        Platform::Linux | Platform::Unknown => home.join(".config/Code"),
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

    // android avd images — emulator snapshots, can be 4-8 GB each
    // only flag the avd subdirectory, not ~/.android root (contains keychains/device tokens)
    locations.push(CacheLocation {
        name: "android AVD images",
        path: home.join(".android/avd"),
        category: BloatCategory::IdeData,
        cleanup_hint: "delete unused emulators via Android Studio AVD Manager",
    });

    // android sdk manager download cache
    locations.push(CacheLocation {
        name: "android SDK cache",
        path: home.join(".android/cache"),
        category: BloatCategory::IdeData,
        cleanup_hint: "safe to delete, re-downloaded on next Android Studio sync",
    });

    // android sdk — platform-specific install location
    let android_sdk_path = match platform {
        Platform::MacOS => home.join("Library/Android/sdk"),
        Platform::Windows => home
            .join("AppData")
            .join("Local")
            .join("Android")
            .join("Sdk"),
        Platform::Linux | Platform::Unknown => home.join("Android/Sdk"),
    };
    locations.push(CacheLocation {
        name: "android SDK",
        path: android_sdk_path,
        category: BloatCategory::IdeData,
        cleanup_hint: "remove unused SDK versions via Android Studio SDK Manager",
    });

    (locations, diagnostics)
}

fn get_homebrew_cache(timeout: Duration) -> Result<Option<PathBuf>, String> {
    use std::io::Read;
    use std::process::Stdio;

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

    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let mut stderr = String::new();
                    if let Some(mut stderr_pipe) = child.stderr.take() {
                        let _ = stderr_pipe.read_to_string(&mut stderr);
                    }
                    return Err(format!(
                        "brew --cache failed with status {}: {}",
                        status.code().unwrap_or(-1),
                        stderr.trim()
                    ));
                }

                let mut output = String::new();
                let mut stdout = child
                    .stdout
                    .take()
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
                    return Err(format!(
                        "brew returned path {} but it doesn't exist",
                        path.display()
                    ));
                }
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    // wait for process to actually terminate to avoid zombie process
                    let _ = child.wait();
                    return Err(format!(
                        "brew --cache timed out after {} seconds",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!("failed to wait for brew process: {e}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn locations(platform: Platform) -> Vec<CacheLocation> {
        let home = PathBuf::from("/home/testuser");
        let (locs, _) = get_cache_locations(&home, platform, Duration::from_secs(5));
        locs
    }

    fn find<'a>(locs: &'a [CacheLocation], name: &str) -> Option<&'a CacheLocation> {
        locs.iter().find(|l| l.name == name)
    }

    // ── android paths ────────────────────────────────────────────────────────

    #[test]
    fn android_avd_path_present_on_linux() {
        let locs = locations(Platform::Linux);
        let avd = find(&locs, "android AVD images").unwrap();
        assert_eq!(avd.path, PathBuf::from("/home/testuser/.android/avd"));
        assert_eq!(avd.category, BloatCategory::IdeData);
    }

    #[test]
    fn android_sdk_cache_path_present() {
        let locs = locations(Platform::Linux);
        let cache = find(&locs, "android SDK cache").unwrap();
        assert_eq!(cache.path, PathBuf::from("/home/testuser/.android/cache"));
    }

    #[test]
    fn android_sdk_path_linux() {
        let locs = locations(Platform::Linux);
        let sdk = find(&locs, "android SDK").unwrap();
        assert_eq!(sdk.path, PathBuf::from("/home/testuser/Android/Sdk"));
    }

    #[test]
    fn android_sdk_path_macos() {
        let home = PathBuf::from("/Users/testuser");
        let (locs, _) = get_cache_locations(&home, Platform::MacOS, Duration::from_secs(5));
        let sdk = find(&locs, "android SDK").unwrap();
        assert_eq!(
            sdk.path,
            PathBuf::from("/Users/testuser/Library/Android/sdk")
        );
    }

    #[test]
    fn android_sdk_path_windows() {
        let home = PathBuf::from("C:\\Users\\testuser");
        let (locs, _) = get_cache_locations(&home, Platform::Windows, Duration::from_secs(5));
        let sdk = find(&locs, "android SDK").unwrap();
        assert_eq!(
            sdk.path,
            PathBuf::from("C:\\Users\\testuser")
                .join("AppData")
                .join("Local")
                .join("Android")
                .join("Sdk")
        );
    }

    // ── platform-specific paths ───────────────────────────────────────────────

    #[test]
    fn macos_pip_uses_library_caches() {
        let home = PathBuf::from("/Users/testuser");
        let (locs, _) = get_cache_locations(&home, Platform::MacOS, Duration::from_secs(5));
        let pip = find(&locs, "pip cache").unwrap();
        assert!(pip.path.to_string_lossy().contains("Library/Caches"));
    }

    #[test]
    fn linux_pip_uses_dot_cache() {
        let locs = locations(Platform::Linux);
        let pip = find(&locs, "pip cache").unwrap();
        assert!(pip.path.to_string_lossy().contains(".cache/pip"));
    }

    #[test]
    fn macos_yarn_uses_library_caches() {
        let home = PathBuf::from("/Users/testuser");
        let (locs, _) = get_cache_locations(&home, Platform::MacOS, Duration::from_secs(5));
        let yarn = find(&locs, "yarn cache").unwrap();
        assert!(yarn.path.to_string_lossy().contains("Library/Caches"));
    }

    // ── categories ───────────────────────────────────────────────────────────

    #[test]
    fn all_android_entries_are_ide_data() {
        let locs = locations(Platform::Linux);
        for name in &["android AVD images", "android SDK cache", "android SDK"] {
            let loc = find(&locs, name).unwrap();
            assert_eq!(
                loc.category,
                BloatCategory::IdeData,
                "{name} should be IdeData"
            );
        }
    }
}
