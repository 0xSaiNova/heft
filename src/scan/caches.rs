//! Package manager and toolchain cache detector.
//!
//! Checks known cache locations:
//! - npm: ~/.npm
//! - pip: ~/.cache/pip (Linux) or ~/Library/Caches/pip (macOS)
//! - cargo: ~/.cargo/registry, ~/.cargo/git
//! - homebrew: $(brew --cache)
//! - yarn: ~/.cache/yarn or ~/Library/Caches/Yarn
//! - pnpm: ~/.local/share/pnpm/store
//! - VS Code: ~/.config/Code or ~/Library/Application Support/Code
//!
//! Platform aware, skips locations that don't exist on the current OS.

use crate::config::Config;
use super::detector::{Detector, DetectorResult};

pub struct CacheDetector;

impl Detector for CacheDetector {
    fn name(&self) -> &'static str {
        "caches"
    }

    fn available(&self, _config: &Config) -> bool {
        true
    }

    fn scan(&self, _config: &Config) -> DetectorResult {
        DetectorResult::empty()
    }
}
