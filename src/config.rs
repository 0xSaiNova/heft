use std::path::PathBuf;
use std::time::Duration;

use crate::cli::{CleanArgs, ScanArgs};
use crate::platform::{self, Platform};

pub struct Config {
    pub roots: Vec<PathBuf>,
    pub timeout: Duration,
    pub skip_docker: bool,
    pub json_output: bool,
    pub verbose: bool,
    pub progressive: bool,
    pub platform: Platform,
}

impl Config {
    pub fn from_scan_args(args: &ScanArgs) -> Self {
        let platform = platform::detect();

        let roots = args
            .roots
            .clone()
            .unwrap_or_else(|| platform::home_dir().map(|h| vec![h]).unwrap_or_default());

        Config {
            roots,
            timeout: Duration::from_secs(args.timeout),
            skip_docker: args.no_docker,
            json_output: args.json,
            verbose: args.verbose,
            progressive: args.progressive,
            platform,
        }
    }

    pub fn from_clean_args(args: &CleanArgs) -> Self {
        let platform = platform::detect();

        let roots = args
            .roots
            .clone()
            .unwrap_or_else(|| platform::home_dir().map(|h| vec![h]).unwrap_or_default());

        Config {
            roots,
            timeout: Duration::from_secs(args.timeout),
            skip_docker: args.no_docker,
            json_output: false,
            verbose: args.verbose,
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
            skip_docker: false,
            json_output: false,
            verbose: false,
            progressive: false,
            platform,
        }
    }
}
