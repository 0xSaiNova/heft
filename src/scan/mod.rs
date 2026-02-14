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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub detector_timings: Vec<(String, u128)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_memory_bytes: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub detector_memory: Vec<(String, usize)>,
}

impl ScanResult {
    pub fn empty() -> Self {
        ScanResult {
            entries: Vec::new(),
            diagnostics: Vec::new(),
            duration_ms: None,
            detector_timings: Vec::new(),
            peak_memory_bytes: None,
            detector_memory: Vec::new(),
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

    // Reserve space for per-detector metrics
    scan_result.detector_timings.reserve(detectors.len());
    scan_result.detector_memory.reserve(detectors.len());

    // Track peak memory across entire scan
    let mut peak_memory: usize = 0;

    // Sample baseline memory before detectors run
    if let Some(usage) = memory_stats::memory_stats() {
        peak_memory = usage.physical_mem;
    }

    for detector in detectors {
        let detector_name = detector.name();

        // Skip unavailable detectors
        if !detector.available(config) {
            let msg = format!(
                "{}: skipped (not available on this platform)",
                detector_name
            );
            if config.progressive {
                eprintln!("{msg}");
            }
            scan_result.diagnostics.push(msg);
            continue;
        }

        // Show start message in progressive mode
        if config.progressive {
            eprintln!("Scanning {}...", detector_name);
        }

        // Sample memory before detector runs
        let memory_before = memory_stats::memory_stats()
            .map(|usage| usage.physical_mem)
            .unwrap_or(0);

        // Run detector and measure timing
        let detector_start = std::time::Instant::now();
        let result = detector.scan(config);
        let detector_duration = detector_start.elapsed();

        // Sample memory after detector completes
        let memory_after = memory_stats::memory_stats()
            .map(|usage| usage.physical_mem)
            .unwrap_or(0);

        // Calculate per-detector memory delta (can be negative if GC ran)
        // Use saturating_sub to avoid underflow, delta represents growth
        let memory_delta = memory_after.saturating_sub(memory_before);

        // Update global peak with current RSS
        peak_memory = peak_memory.max(memory_after);

        // Store per-detector metrics
        scan_result.detector_timings.push((detector_name.to_string(), detector_duration.as_millis()));
        scan_result.detector_memory.push((detector_name.to_string(), memory_delta));

        // Show completion message in progressive mode
        if config.progressive {
            let count = result.entries.len();
            let total_bytes: u64 = result.entries.iter().map(|e| e.size_bytes).sum();
            eprintln!(
                "{} complete: {} items, {}, {:.2}s",
                detector_name,
                count,
                format_bytes(total_bytes),
                detector_duration.as_secs_f64()
            );
        }

        scan_result.merge(result);
    }

    scan_result.duration_ms = Some(start.elapsed().as_millis());

    // Store peak memory if we successfully sampled at least once
    if peak_memory > 0 {
        scan_result.peak_memory_bytes = Some(peak_memory);
    }

    scan_result
}
