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
    // Use Option to distinguish between "no memory tracking" and "0 bytes used"
    let mut peak_memory: Option<usize> = None;

    // Sample baseline memory before detectors run
    if let Some(usage) = memory_stats::memory_stats() {
        peak_memory = Some(usage.physical_mem);
    }

    for detector in detectors {
        let detector_name = detector.name();

        // Skip unavailable detectors
        if !detector.available(config) {
            let msg = format!("{detector_name}: skipped (not available on this platform)");
            if config.progressive {
                eprintln!("{msg}");
            }
            scan_result.diagnostics.push(msg);
            continue;
        }

        // Show start message in progressive mode
        if config.progressive {
            eprintln!("Scanning {detector_name}...");
        }

        // Run detector and measure timing
        let detector_start = std::time::Instant::now();
        let result = detector.scan(config);
        let detector_duration = detector_start.elapsed();

        // Store timing (always available)
        scan_result.detector_timings.push((detector_name.to_string(), detector_duration.as_millis()));

        // Sample memory only if baseline sampling succeeded
        if peak_memory.is_some() {
            let memory_before = memory_stats::memory_stats()
                .map(|usage| usage.physical_mem)
                .unwrap_or(0);

            let memory_after = memory_stats::memory_stats()
                .map(|usage| usage.physical_mem)
                .unwrap_or(0);

            // Calculate per-detector memory delta
            // saturating_sub returns 0 if memory decreased (e.g. GC ran during detector)
            // This represents memory growth attributed to the detector
            let memory_delta = memory_after.saturating_sub(memory_before);

            // Update global peak with current RSS
            if let Some(current_peak) = peak_memory {
                peak_memory = Some(current_peak.max(memory_after));
            }

            scan_result.detector_memory.push((detector_name.to_string(), memory_delta));
        }

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

    // Store peak memory if sampling was available
    scan_result.peak_memory_bytes = peak_memory;

    scan_result
}
