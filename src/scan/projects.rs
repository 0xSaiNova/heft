//! Project artifact detector.
//!
//! Walks configured root directories looking for known build artifact patterns:
//! - node_modules (Node.js)
//! - target/ (Rust/Cargo)
//! - __pycache__, .venv, venv (Python)
//! - vendor/ (Go, PHP)
//! - build/, dist/ (various)
//! - DerivedData (Xcode)
//!
//! Reports per-project size and last modification time of source files.

use crate::config::Config;
use super::detector::{Detector, DetectorResult};

pub struct ProjectDetector;

impl Detector for ProjectDetector {
    fn name(&self) -> &'static str {
        "projects"
    }

    fn available(&self, _config: &Config) -> bool {
        true
    }

    fn scan(&self, _config: &Config) -> DetectorResult {
        DetectorResult::empty()
    }
}
