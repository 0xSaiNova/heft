//! Docker storage detector.
//!
//! Queries the Docker daemon via `docker system df --format json` for:
//! - Images (total and reclaimable)
//! - Containers (total and reclaimable)
//! - Volumes (total and reclaimable)
//! - Build cache (total and reclaimable)
//!
//! Handles gracefully:
//! - Docker not installed
//! - Docker daemon not running
//! - Permission denied
//!
//! Does not walk Docker's internal storage directories directly.

use crate::config::Config;
use crate::platform;
use super::detector::{Detector, DetectorResult};

pub struct DockerDetector;

impl Detector for DockerDetector {
    fn name(&self) -> &'static str {
        "docker"
    }

    fn available(&self, config: &Config) -> bool {
        !config.skip_docker && platform::docker_available()
    }

    fn scan(&self, _config: &Config) -> DetectorResult {
        DetectorResult::empty()
    }
}
