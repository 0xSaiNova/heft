pub mod table;
pub mod json;

use crate::config::Config;
use crate::scan::ScanResult;
use crate::util::format_bytes;

pub fn print(result: &ScanResult, config: &Config) {
    if config.json_output {
        println!("{}", json::render(result));
    } else {
        print!("{}", table::render(result));
        print_scan_info(result, config.verbose);
        print_diagnostics(result, config.verbose);
    }
}

fn print_scan_info(result: &ScanResult, verbose: bool) {
    if let Some(duration_ms) = result.duration_ms {
        let duration_sec = duration_ms as f64 / 1000.0;
        println!("\nscan completed in {duration_sec:.2}s");

        // Display peak memory in verbose mode or JSON (always tracked)
        if verbose {
            if let Some(peak_bytes) = result.peak_memory_bytes {
                println!("peak memory: {}", format_bytes(peak_bytes as u64));
            }
        }

        // Display per-detector metrics in verbose mode
        if verbose && !result.detector_timings.is_empty() {
            println!("\ndetector timing:");

            for (detector_name, timing_ms) in &result.detector_timings {
                let timing_sec = *timing_ms as f64 / 1000.0;

                // Linear search for memory delta - only 3 detectors, faster than HashMap
                let memory_delta = result.detector_memory
                    .iter()
                    .find(|(name, _)| name == detector_name)
                    .map(|(_, delta)| *delta);

                // Show memory delta if available for this detector
                if let Some(delta) = memory_delta {
                    println!(
                        "  {detector_name}: {timing_sec:.2}s, {}",
                        format_bytes(delta as u64)
                    );
                } else {
                    println!("  {detector_name}: {timing_sec:.2}s");
                }
            }
        }
    }
}

fn print_diagnostics(result: &ScanResult, verbose: bool) {
    if result.diagnostics.is_empty() {
        return;
    }

    println!();
    if verbose {
        println!("Diagnostics:");
        println!("{}", "-".repeat(40));
        for diagnostic in &result.diagnostics {
            println!("  {diagnostic}");
        }
    } else {
        for diagnostic in &result.diagnostics {
            println!("[diagnostic] {diagnostic}");
        }
    }
}
