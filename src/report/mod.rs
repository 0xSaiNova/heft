pub mod table;
pub mod json;

use crate::config::Config;
use crate::scan::ScanResult;

pub fn print(result: &ScanResult, config: &Config) {
    if config.json_output {
        println!("{}", json::render(result));
    } else {
        print!("{}", table::render(result));
        print_diagnostics(result);
    }
}

fn print_diagnostics(result: &ScanResult) {
    if !result.diagnostics.is_empty() {
        println!();
        for diagnostic in &result.diagnostics {
            println!("[diagnostic] {diagnostic}");
        }
    }
}
