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
                    let reclaimable = if cache.not_reclaimable { 0 } else { size };
                    entries.push(BloatEntry {
                        category: cache.category,
                        name: cache.name.clone(),
                        location: Location::FilesystemPath(cache.path.clone()),
                        size_bytes: size,
                        reclaimable_bytes: reclaimable,
                        last_modified: None,
                        cleanup_hint: Some(cache.cleanup_hint.clone()),
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

// String fields so WSL entries can include dynamic names (distro package name).
struct CacheLocation {
    name: String,
    path: PathBuf,
    category: BloatCategory,
    cleanup_hint: String,
    /// When true, size is reported but reclaimable_bytes is 0 (e.g. WSL VHDX disks).
    not_reclaimable: bool,
}

impl CacheLocation {
    fn new(
        name: &'static str,
        path: PathBuf,
        category: BloatCategory,
        cleanup_hint: &'static str,
    ) -> Self {
        CacheLocation {
            name: name.to_string(),
            path,
            category,
            cleanup_hint: cleanup_hint.to_string(),
            not_reclaimable: false,
        }
    }
}

fn get_cache_locations(
    home: &Path,
    platform: Platform,
    timeout: Duration,
) -> (Vec<CacheLocation>, Vec<String>) {
    let mut locations = Vec::new();
    let mut diagnostics = Vec::new();

    // npm cache
    locations.push(CacheLocation::new(
        "npm cache",
        home.join(".npm"),
        BloatCategory::PackageCache,
        "npm cache clean --force",
    ));

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
    locations.push(CacheLocation::new(
        "yarn cache",
        yarn_path,
        BloatCategory::PackageCache,
        "yarn cache clean",
    ));

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
    locations.push(CacheLocation::new(
        "pnpm store",
        pnpm_path,
        BloatCategory::PackageCache,
        "pnpm store prune",
    ));

    // pip cache
    let pip_path = match platform {
        Platform::MacOS => home.join("Library/Caches/pip"),
        Platform::Windows => home.join("AppData").join("Local").join("pip").join("Cache"),
        Platform::Linux | Platform::Unknown => home.join(".cache/pip"),
    };
    locations.push(CacheLocation::new(
        "pip cache",
        pip_path,
        BloatCategory::PackageCache,
        "pip cache purge",
    ));

    // cargo registry and git checkouts
    locations.push(CacheLocation::new(
        "cargo registry",
        home.join(".cargo/registry"),
        BloatCategory::PackageCache,
        "cargo cache --autoclean (requires cargo-cache)",
    ));
    locations.push(CacheLocation::new(
        "cargo git",
        home.join(".cargo/git"),
        BloatCategory::PackageCache,
        "cargo cache --autoclean (requires cargo-cache)",
    ));

    // homebrew cache (macOS and Linux)
    match get_homebrew_cache(timeout) {
        Ok(Some(brew_cache)) => {
            locations.push(CacheLocation::new(
                "homebrew cache",
                brew_cache,
                BloatCategory::PackageCache,
                "brew cleanup",
            ));
        }
        Ok(None) => {
            // brew not installed, this is normal
        }
        Err(e) => {
            diagnostics.push(format!("homebrew cache detection failed: {e}"));
        }
    }

    // go module cache
    locations.push(CacheLocation::new(
        "go module cache",
        home.join("go/pkg/mod"),
        BloatCategory::PackageCache,
        "go clean -modcache",
    ));

    // VS Code extensions and cache
    let vscode_path = match platform {
        Platform::MacOS => home.join("Library/Application Support/Code"),
        Platform::Windows => home.join("AppData").join("Roaming").join("Code"),
        Platform::Linux | Platform::Unknown => home.join(".config/Code"),
    };
    locations.push(CacheLocation::new(
        "vscode data",
        vscode_path,
        BloatCategory::IdeData,
        "clear from within vscode or delete unused extensions",
    ));

    // gradle cache — cross-platform dotfile path, same on all OSes
    locations.push(CacheLocation::new(
        "gradle cache",
        home.join(".gradle/caches"),
        BloatCategory::PackageCache,
        "safe to delete, rebuilt on next gradle build",
    ));

    // maven cache
    locations.push(CacheLocation::new(
        "maven cache",
        home.join(".m2/repository"),
        BloatCategory::PackageCache,
        "mvn dependency:purge-local-repository",
    ));

    // nuget package cache — cross-platform dotfile path, most relevant on Windows
    locations.push(CacheLocation::new(
        "nuget cache",
        home.join(".nuget").join("packages"),
        BloatCategory::PackageCache,
        "dotnet nuget locals all --clear",
    ));

    // android avd images — emulator snapshots, can be 4-8 GB each
    // only flag the avd subdirectory, not ~/.android root (contains keychains/device tokens)
    locations.push(CacheLocation::new(
        "android AVD images",
        home.join(".android/avd"),
        BloatCategory::IdeData,
        "delete unused emulators via Android Studio AVD Manager",
    ));

    // android sdk manager download cache
    locations.push(CacheLocation::new(
        "android SDK cache",
        home.join(".android/cache"),
        BloatCategory::IdeData,
        "safe to delete, re-downloaded on next Android Studio sync",
    ));

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
    locations.push(CacheLocation::new(
        "android SDK",
        android_sdk_path,
        BloatCategory::IdeData,
        "remove unused SDK versions via Android Studio SDK Manager",
    ));

    // WSL2 virtual disk detection — when running inside WSL2, the distro's
    // ext4.vhdx grows as files are written but never shrinks automatically.
    // Windows drives are mounted at /mnt/c, so we can read AppData paths.
    // WSL_INTEROP is set exclusively by WSL2 (not WSL1), so this is safe.
    if platform::is_wsl() {
        match wsl_windows_username() {
            Ok(win_user) => {
                let win_local = PathBuf::from("/mnt/c/Users")
                    .join(&win_user)
                    .join("AppData/Local");

                // Docker Desktop WSL2 disks — path varies by version
                for docker_rel in &["Docker/wsl/data/ext4.vhdx", "Docker/wsl/distro/ext4.vhdx"] {
                    let vhdx = win_local.join(docker_rel);
                    if vhdx.exists() {
                        locations.push(CacheLocation {
                            name: "docker desktop WSL2 disk".to_string(),
                            path: vhdx,
                            category: BloatCategory::ContainerData,
                            cleanup_hint: "run 'wsl --shutdown' then compact with 'Optimize-VHD' in PowerShell (admin)".to_string(),
                            not_reclaimable: true,
                        });
                    }
                }

                // WSL distro virtual disks — scan all Packages entries for ext4.vhdx
                // rather than filtering by publisher prefix (Debian, Kali, openSUSE etc.
                // all use different prefixes but the vhdx path is consistent).
                let packages_dir = win_local.join("Packages");
                if let Ok(entries) = std::fs::read_dir(&packages_dir) {
                    for entry in entries.flatten() {
                        let vhdx = entry.path().join("LocalState/ext4.vhdx");
                        if vhdx.exists() {
                            let pkg_name = entry.file_name().to_string_lossy().into_owned();
                            locations.push(CacheLocation {
                                name: format!("WSL2 distro disk ({pkg_name})"),
                                path: vhdx,
                                category: BloatCategory::SystemCache,
                                cleanup_hint: "run 'wsl --shutdown' then 'wsl --manage <distro> --set-sparse true' to enable sparse VHD".to_string(),
                                not_reclaimable: true,
                            });
                        }
                    }
                }
            }
            Err(msg) => {
                diagnostics.push(msg);
            }
        }
    }

    (locations, diagnostics)
}

/// Resolves the Windows username when running inside WSL2.
/// Returns an error string (suitable for diagnostics) if it cannot be determined safely.
fn wsl_windows_username() -> Result<String, String> {
    // /mnt/c/Users may not be mounted if the Windows drive is unavailable
    let users_dir = PathBuf::from("/mnt/c/Users");
    if !users_dir.exists() {
        return Err(
            "WSL2 detected but /mnt/c/Users is not mounted — skipping Windows disk detection"
                .to_string(),
        );
    }

    let system_names = ["Public", "Default", "Default User", "All Users"];

    let candidates: Vec<String> = std::fs::read_dir(&users_dir)
        .map_err(|e| format!("WSL2: could not read /mnt/c/Users: {e}"))?
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| !system_names.contains(&name.as_str()))
        .collect();

    match candidates.len() {
        0 => Err("WSL2: no user directories found under /mnt/c/Users".to_string()),
        1 => Ok(candidates.into_iter().next().unwrap()),
        _ => {
            // multiple users — ask Windows directly via WSL interop
            wsl_username_via_cmd()
                .and_then(|name| {
                    // sanity check: the name should match one of the dirs we found
                    if candidates.contains(&name) {
                        Ok(name)
                    } else {
                        Err(format!(
                            "WSL2: cmd.exe returned '{name}' but that directory doesn't exist under /mnt/c/Users"
                        ))
                    }
                })
                .map_err(|e| {
                    format!(
                        "WSL2: multiple Windows users found ({}) and fallback failed: {}",
                        candidates.join(", "),
                        e
                    )
                })
        }
    }
}

/// Asks Windows for the current username via `cmd.exe /c echo %USERNAME%`.
/// Only works when WSL interop is enabled (the default).
fn wsl_username_via_cmd() -> Result<String, String> {
    let output = Command::new("cmd.exe")
        .args(["/c", "echo", "%USERNAME%"])
        .output()
        .map_err(|e| format!("failed to run cmd.exe: {e}"))?;

    if !output.status.success() {
        return Err("cmd.exe returned non-zero exit code".to_string());
    }

    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() || name == "%USERNAME%" {
        return Err("cmd.exe returned empty or unexpanded username".to_string());
    }

    Ok(name)
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

    // ── wsl username resolution ───────────────────────────────────────────────

    #[test]
    fn wsl_username_errors_when_no_mount() {
        // skip in WSL2 where /mnt/c/Users is actually mounted
        if platform::is_wsl() {
            return;
        }
        let result = wsl_windows_username();
        assert!(result.is_err());
    }

    // ── nuget cache ───────────────────────────────────────────────────────────

    #[test]
    fn nuget_cache_present_on_all_platforms() {
        for platform in [Platform::Linux, Platform::MacOS, Platform::Windows] {
            let home = PathBuf::from("/home/testuser");
            let (locs, _) = get_cache_locations(&home, platform, Duration::from_secs(5));
            assert!(
                find(&locs, "nuget cache").is_some(),
                "nuget cache missing on {platform:?}"
            );
        }
    }
}
