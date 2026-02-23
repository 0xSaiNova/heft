use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use directories::BaseDirs;
use serde::Deserialize;

use crate::cli::{CleanArgs, ScanArgs};
use crate::platform::{self, Platform};

// ---------------------------------------------------------------------------
// File config (~/.config/heft/config.toml)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
struct FileScanConfig {
    roots: Option<Vec<PathBuf>>,
    timeout: Option<u64>,
    json: Option<bool>,
    verbose: Option<bool>,
    progressive: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct FileDetectorsConfig {
    docker: Option<bool>,
    xcode: Option<bool>,
    projects: Option<bool>,
    caches: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    #[serde(default)]
    scan: FileScanConfig,
    #[serde(default)]
    detectors: FileDetectorsConfig,
}

fn load_file_config() -> Option<FileConfig> {
    let base = BaseDirs::new()?;
    let path = base.config_dir().join("heft").join("config.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    match toml::from_str(&content) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!(
                "warning: failed to parse config file {}: {e}",
                path.display()
            );
            None
        }
    }
}

/// Collect detector names disabled by the file config.
fn disabled_from_file(det: &FileDetectorsConfig) -> HashSet<String> {
    let mut out = HashSet::new();
    if det.docker == Some(false) {
        out.insert("docker".to_string());
    }
    if det.xcode == Some(false) {
        out.insert("xcode".to_string());
    }
    if det.projects == Some(false) {
        out.insert("projects".to_string());
    }
    if det.caches == Some(false) {
        out.insert("caches".to_string());
    }
    out
}

// ---------------------------------------------------------------------------
// Runtime config
// ---------------------------------------------------------------------------

pub struct Config {
    pub roots: Vec<PathBuf>,
    pub timeout: Duration,
    pub disabled_detectors: HashSet<String>,
    pub json_output: bool,
    pub verbose: bool,
    pub progressive: bool,
    pub platform: Platform,
}

impl Config {
    pub fn is_detector_enabled(&self, name: &str) -> bool {
        !self.disabled_detectors.contains(name)
    }

    pub fn from_scan_args(args: &ScanArgs) -> Self {
        let file = load_file_config().unwrap_or_default();
        Self::merge_scan(args, &file)
    }

    fn merge_scan(args: &ScanArgs, file: &FileConfig) -> Self {
        let platform = platform::detect();

        // roots: CLI > file > home dir
        let roots = args
            .roots
            .clone()
            .or(file.scan.roots.clone())
            .unwrap_or_else(|| platform::home_dir().map(|h| vec![h]).unwrap_or_default());

        // timeout: CLI > file > default 30s
        let timeout = args.timeout.or(file.scan.timeout).unwrap_or(30);

        // booleans: --flag forces on, --no-flag forces off, otherwise file config
        let json_output = if args.no_json {
            false
        } else if args.json {
            true
        } else {
            file.scan.json.unwrap_or(false)
        };
        let verbose = if args.no_verbose {
            false
        } else if args.verbose {
            true
        } else {
            file.scan.verbose.unwrap_or(false)
        };
        let progressive = if args.no_progressive {
            false
        } else if args.progressive {
            true
        } else {
            file.scan.progressive.unwrap_or(false)
        };

        // disabled detectors: file config base, then CLI --no-docker / --disable
        let mut disabled = disabled_from_file(&file.detectors);
        if args.no_docker {
            disabled.insert("docker".to_string());
        }
        if let Some(ref names) = args.disable {
            disabled.extend(names.iter().cloned());
        }

        Config {
            roots,
            timeout: Duration::from_secs(timeout),
            disabled_detectors: disabled,
            json_output,
            verbose,
            progressive,
            platform,
        }
    }

    pub fn from_clean_args(args: &CleanArgs) -> Self {
        let platform = platform::detect();
        let file = load_file_config().unwrap_or_default();

        let roots = args
            .roots
            .clone()
            .or(file.scan.roots)
            .unwrap_or_else(|| platform::home_dir().map(|h| vec![h]).unwrap_or_default());

        let timeout = args.timeout.or(file.scan.timeout).unwrap_or(30);
        let verbose = if args.no_verbose {
            false
        } else if args.verbose {
            true
        } else {
            file.scan.verbose.unwrap_or(false)
        };

        let mut disabled = disabled_from_file(&file.detectors);
        if args.no_docker {
            disabled.insert("docker".to_string());
        }
        if let Some(ref names) = args.disable {
            disabled.extend(names.iter().cloned());
        }

        Config {
            roots,
            timeout: Duration::from_secs(timeout),
            disabled_detectors: disabled,
            json_output: false,
            verbose,
            progressive: false,
            platform,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let platform = platform::detect();
        let roots = platform::home_dir().map(|h| vec![h]).unwrap_or_default();

        Config {
            roots,
            timeout: Duration::from_secs(30),
            disabled_detectors: HashSet::new(),
            json_output: false,
            verbose: false,
            progressive: false,
            platform,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ScanArgs;

    fn default_scan_args() -> ScanArgs {
        ScanArgs {
            roots: None,
            json: false,
            no_json: false,
            no_docker: false,
            disable: None,
            timeout: None,
            verbose: false,
            no_verbose: false,
            progressive: false,
            no_progressive: false,
        }
    }

    // ── disabled_from_file ──────────────────────────────────────────────────

    #[test]
    fn disabled_from_file_empty_config() {
        let det = FileDetectorsConfig::default();
        assert!(disabled_from_file(&det).is_empty());
    }

    #[test]
    fn disabled_from_file_true_does_not_disable() {
        let det = FileDetectorsConfig {
            docker: Some(true),
            xcode: Some(true),
            projects: Some(true),
            caches: Some(true),
        };
        assert!(disabled_from_file(&det).is_empty());
    }

    #[test]
    fn disabled_from_file_false_disables() {
        let det = FileDetectorsConfig {
            docker: Some(false),
            xcode: Some(false),
            projects: None,
            caches: Some(false),
        };
        let disabled = disabled_from_file(&det);
        assert!(disabled.contains("docker"));
        assert!(disabled.contains("xcode"));
        assert!(disabled.contains("caches"));
        assert!(!disabled.contains("projects"));
    }

    // ── merge_scan: timeout precedence ──────────────────────────────────────

    #[test]
    fn timeout_defaults_to_30() {
        let args = default_scan_args();
        let file = FileConfig::default();
        let config = Config::merge_scan(&args, &file);
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn timeout_file_overrides_default() {
        let args = default_scan_args();
        let file = FileConfig {
            scan: FileScanConfig {
                timeout: Some(60),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::merge_scan(&args, &file);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn timeout_cli_overrides_file() {
        let args = ScanArgs {
            timeout: Some(10),
            ..default_scan_args()
        };
        let file = FileConfig {
            scan: FileScanConfig {
                timeout: Some(60),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::merge_scan(&args, &file);
        assert_eq!(config.timeout, Duration::from_secs(10));
    }

    // ── merge_scan: boolean flags ───────────────────────────────────────────

    #[test]
    fn verbose_defaults_to_false() {
        let config = Config::merge_scan(&default_scan_args(), &FileConfig::default());
        assert!(!config.verbose);
    }

    #[test]
    fn verbose_file_turns_on() {
        let file = FileConfig {
            scan: FileScanConfig {
                verbose: Some(true),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::merge_scan(&default_scan_args(), &file);
        assert!(config.verbose);
    }

    #[test]
    fn no_verbose_overrides_file() {
        let args = ScanArgs {
            no_verbose: true,
            ..default_scan_args()
        };
        let file = FileConfig {
            scan: FileScanConfig {
                verbose: Some(true),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::merge_scan(&args, &file);
        assert!(!config.verbose);
    }

    #[test]
    fn json_flag_overrides_file_false() {
        let args = ScanArgs {
            json: true,
            ..default_scan_args()
        };
        let file = FileConfig {
            scan: FileScanConfig {
                json: Some(false),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::merge_scan(&args, &file);
        assert!(config.json_output);
    }

    #[test]
    fn no_json_overrides_file_true() {
        let args = ScanArgs {
            no_json: true,
            ..default_scan_args()
        };
        let file = FileConfig {
            scan: FileScanConfig {
                json: Some(true),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::merge_scan(&args, &file);
        assert!(!config.json_output);
    }

    // ── merge_scan: disabled detectors ──────────────────────────────────────

    #[test]
    fn no_docker_flag_disables_docker() {
        let args = ScanArgs {
            no_docker: true,
            ..default_scan_args()
        };
        let config = Config::merge_scan(&args, &FileConfig::default());
        assert!(config.disabled_detectors.contains("docker"));
    }

    #[test]
    fn disable_flag_disables_listed() {
        let args = ScanArgs {
            disable: Some(vec!["xcode".to_string(), "caches".to_string()]),
            ..default_scan_args()
        };
        let config = Config::merge_scan(&args, &FileConfig::default());
        assert!(config.disabled_detectors.contains("xcode"));
        assert!(config.disabled_detectors.contains("caches"));
        assert!(!config.disabled_detectors.contains("docker"));
    }

    #[test]
    fn file_and_cli_disabled_merge() {
        let args = ScanArgs {
            no_docker: true,
            ..default_scan_args()
        };
        let file = FileConfig {
            detectors: FileDetectorsConfig {
                xcode: Some(false),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::merge_scan(&args, &file);
        assert!(config.disabled_detectors.contains("docker"));
        assert!(config.disabled_detectors.contains("xcode"));
    }

    // ── merge_scan: roots precedence ────────────────────────────────────────

    #[test]
    fn roots_cli_overrides_file() {
        let args = ScanArgs {
            roots: Some(vec![PathBuf::from("/cli/path")]),
            ..default_scan_args()
        };
        let file = FileConfig {
            scan: FileScanConfig {
                roots: Some(vec![PathBuf::from("/file/path")]),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::merge_scan(&args, &file);
        assert_eq!(config.roots, vec![PathBuf::from("/cli/path")]);
    }

    #[test]
    fn roots_file_overrides_default() {
        let args = default_scan_args();
        let file = FileConfig {
            scan: FileScanConfig {
                roots: Some(vec![PathBuf::from("/file/path")]),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::merge_scan(&args, &file);
        assert_eq!(config.roots, vec![PathBuf::from("/file/path")]);
    }
}
