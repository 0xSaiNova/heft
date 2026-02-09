pub mod detector;
pub mod projects;
pub mod caches;
pub mod docker;

use serde::Serialize;

use crate::config::Config;
use detector::{Detector, DetectorResult, BloatEntry};

#[derive(Serialize)]
pub struct ScanResult {
    pub entries: Vec<BloatEntry>,
    pub diagnostics: Vec<String>,
}

impl ScanResult {
    pub fn empty() -> Self {
        ScanResult {
            entries: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn merge(&mut self, result: DetectorResult) {
        self.entries.extend(result.entries);
        self.diagnostics.extend(result.diagnostics);
    }
}

pub fn run(config: &Config) -> ScanResult {
    let mut scan_result = ScanResult::empty();

    let detectors: Vec<Box<dyn Detector>> = vec![
        Box::new(projects::ProjectDetector),
        Box::new(caches::CacheDetector),
        Box::new(docker::DockerDetector),
    ];

    for detector in detectors {
        if !detector.available(config) {
            scan_result.diagnostics.push(format!(
                "{}: skipped (not available on this platform)",
                detector.name()
            ));
            continue;
        }

        let result = detector.scan(config);
        scan_result.merge(result);
    }

    scan_result
}
