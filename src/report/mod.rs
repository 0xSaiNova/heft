pub mod table;
pub mod json;

use crate::config::Config;
use crate::scan::ScanResult;

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

        if verbose && !result.detector_timings.is_empty() {
            for (detector_name, timing_ms) in &result.detector_timings {
                let timing_sec = *timing_ms as f64 / 1000.0;
                println!("  {detector_name}: {timing_sec:.2}s");
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
