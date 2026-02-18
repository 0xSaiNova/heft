pub mod caches;
pub mod detector;
pub mod docker;
pub mod projects;
pub mod xcode;

use std::path::Path;

use serde::Serialize;
use walkdir::WalkDir;

use crate::config::Config;
use crate::util::format_bytes;
use detector::{BloatEntry, Detector, DetectorResult};

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
        Box::new(xcode::XcodeDetector),
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

        // Sample memory BEFORE detector runs (if tracking enabled)
        let memory_before = if peak_memory.is_some() {
            memory_stats::memory_stats()
                .map(|usage| usage.physical_mem)
                .unwrap_or(0)
        } else {
            0
        };

        // Run detector and measure timing
        let detector_start = std::time::Instant::now();
        let result = detector.scan(config);
        let detector_duration = detector_start.elapsed();

        // Store timing (always available)
        scan_result
            .detector_timings
            .push((detector_name.to_string(), detector_duration.as_millis()));

        // Sample memory AFTER detector completes (if tracking enabled)
        if peak_memory.is_some() {
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

            scan_result
                .detector_memory
                .push((detector_name.to_string(), memory_delta));
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

pub(crate) fn calculate_dir_size(path: &Path) -> Result<(u64, Vec<String>), std::io::Error> {
    let mut total = 0u64;
    let mut warnings = Vec::new();
    let mut overflowed = false;

    for entry in WalkDir::new(path).follow_links(false).into_iter() {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    match entry.metadata() {
                        Ok(metadata) => {
                            let file_size = metadata.len();
                            match total.checked_add(file_size) {
                                Some(new_total) => total = new_total,
                                None => {
                                    if !overflowed {
                                        warnings.push("directory size exceeds u64::MAX, size capped at maximum value".to_string());
                                        overflowed = true;
                                    }
                                    total = u64::MAX;
                                }
                            }
                        }
                        Err(e) => {
                            warnings.push(format!(
                                "failed to read metadata for {}: {}",
                                entry.path().display(),
                                e
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                let path_str = e
                    .path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "unknown path".to_string());

                if e.io_error()
                    .map(|io_err| io_err.kind() == std::io::ErrorKind::PermissionDenied)
                    .unwrap_or(false)
                {
                    warnings.push(format!("permission denied: {path_str}"));
                } else if e.loop_ancestor().is_some() {
                    warnings.push(format!("symlink loop detected: {path_str}"));
                } else {
                    warnings.push(format!("failed to traverse {path_str}: {e}"));
                }
            }
        }
    }

    Ok((total, warnings))
}
