//! Xcode build artifact detector (macOS only).
//!
//! Detects ~/Library/Developer/Xcode/DerivedData, the central location
//! where Xcode stores all compiled build products, indexes, and logs.
//! Can grow to 10-30 GB on active iOS/macOS projects and is fully safe
//! to delete — Xcode rebuilds it on next build.

use crate::config::Config;
use crate::platform::{self, Platform};
use super::detector::{BloatCategory, BloatEntry, Detector, DetectorResult, Location};

pub struct XcodeDetector;

impl Detector for XcodeDetector {
    fn name(&self) -> &'static str {
        "xcode"
    }

    fn available(&self, config: &Config) -> bool {
        config.platform == Platform::MacOS
    }

    fn scan(&self, config: &Config) -> DetectorResult {
        let home = match platform::home_dir() {
            Some(h) => h,
            None => return DetectorResult::with_diagnostic("xcode: could not determine home directory".into()),
        };

        let derived_data = home.join("Library/Developer/Xcode/DerivedData");

        if !derived_data.exists() {
            return DetectorResult::empty();
        }

        match super::calculate_dir_size(&derived_data) {
            Ok((size, warnings)) if size > 0 => {
                let mut diagnostics: Vec<String> = warnings.into_iter()
                    .map(|w| format!("{w} (size may be underestimated)"))
                    .collect();

                if config.verbose {
                    diagnostics.push(format!("xcode: DerivedData at {}", derived_data.display()));
                }

                DetectorResult {
                    entries: vec![BloatEntry {
                        category: BloatCategory::IdeData,
                        name: "Xcode DerivedData".to_string(),
                        location: Location::FilesystemPath(derived_data),
                        size_bytes: size,
                        reclaimable_bytes: size,
                        last_modified: None,
                        cleanup_hint: Some(
                            "safe to delete, Xcode rebuilds on next build. or: Xcode → Settings → Locations → Derived Data → arrow button".to_string()
                        ),
                    }],
                    diagnostics,
                }
            }
            Ok(_) => DetectorResult::empty(),
            Err(e) => DetectorResult::with_diagnostic(
                format!("xcode: failed to calculate DerivedData size: {e}")
            ),
        }
    }
}
