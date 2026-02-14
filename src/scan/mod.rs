pub mod detector;
pub mod projects;
pub mod caches;
pub mod docker;

use serde::Serialize;

use crate::config::Config;
use crate::util::format_bytes;
use detector::{Detector, DetectorResult, BloatEntry};

#[derive(Serialize)]
pub struct ScanResult {
    pub entries: Vec<BloatEntry>,
    pub diagnostics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u128>,
}

impl ScanResult {
    pub fn empty() -> Self {
        ScanResult {
            entries: Vec::new(),
            diagnostics: Vec::new(),
            duration_ms: None,
        }
    }

    fn merge(&mut self, result: DetectorResult) {
        self.entries.extend(result.entries);
        self.diagnostics.extend(result.diagnostics);
    }
}

pub fn run(config: &Config) -> ScanResult {
    let start = std::time::Instant::now();
    let mut scan_result = ScanResult::empty();

    let detectors: Vec<Box<dyn Detector>> = vec![
        Box::new(projects::ProjectDetector),
        Box::new(caches::CacheDetector),
        Box::new(docker::DockerDetector),
    ];

    for detector in detectors {
        if !detector.available(config) {
            let msg = format!(
                "{}: skipped (not available on this platform)",
                detector.name()
            );
            scan_result.diagnostics.push(msg.clone());
            if config.progressive {
                eprintln!("{msg}");
            }
            continue;
        }

        if config.progressive {
            eprintln!("Scanning {}...", detector.name());
        }

        let detector_start = std::time::Instant::now();
        let result = detector.scan(config);
        let detector_duration = detector_start.elapsed();

        if config.progressive {
            let count = result.entries.len();
            let total_bytes: u64 = result.entries.iter().map(|e| e.size_bytes).sum();
            eprintln!(
                "{} complete: {} items, {}, {:.2}s",
                detector.name(),
                count,
                format_bytes(total_bytes),
                detector_duration.as_secs_f64()
            );
        }

        scan_result.merge(result);
    }

    scan_result.duration_ms = Some(start.elapsed().as_millis());
    scan_result
}
