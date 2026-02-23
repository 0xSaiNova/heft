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
            eprintln!("warning: failed to parse config file {}: {e}", path.display());
            None
        }
    }
}

/// Collect detector names disabled by the file config.
fn disabled_from_file(det: &FileDetectorsConfig) -> HashSet<String> {
    let mut out = HashSet::new();
    if det.docker == Some(false) { out.insert("docker".to_string()); }
    if det.xcode == Some(false) { out.insert("xcode".to_string()); }
    if det.projects == Some(false) { out.insert("projects".to_string()); }
    if det.caches == Some(false) { out.insert("caches".to_string()); }
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
        let platform = platform::detect();
        let file = load_file_config().unwrap_or_default();

        // roots: CLI > file > home dir
        let roots = args.roots.clone().or(file.scan.roots).unwrap_or_else(|| {
            platform::home_dir().map(|h| vec![h]).unwrap_or_default()
        });

        // timeout: CLI > file > default 30s
        let timeout = args.timeout.or(file.scan.timeout).unwrap_or(30);

        // booleans: CLI flag wins if true, otherwise fall back to file
        let json_output = args.json || file.scan.json.unwrap_or(false);
        let verbose = args.verbose || file.scan.verbose.unwrap_or(false);
        let progressive = args.progressive || file.scan.progressive.unwrap_or(false);

        // disabled detectors: file config base, --no-docker adds to it
        let mut disabled = disabled_from_file(&file.detectors);
        if args.no_docker {
            disabled.insert("docker".to_string());
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

        let roots = args.roots.clone().or(file.scan.roots).unwrap_or_else(|| {
            platform::home_dir().map(|h| vec![h]).unwrap_or_default()
        });

        let timeout = args.timeout.or(file.scan.timeout).unwrap_or(30);
        let verbose = args.verbose || file.scan.verbose.unwrap_or(false);

        let mut disabled = disabled_from_file(&file.detectors);
        if args.no_docker {
            disabled.insert("docker".to_string());
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
