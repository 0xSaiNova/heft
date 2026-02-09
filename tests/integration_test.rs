use std::path::PathBuf;
use std::time::Duration;

use heft::config::Config;
use heft::platform::Platform;
use heft::scan;

#[test]
fn skeleton_compiles_and_returns_empty_results() {
    let config = Config {
        roots: vec![PathBuf::from("/tmp")],
        timeout: Duration::from_secs(30),
        skip_docker: true,
        json_output: false,
        platform: Platform::Linux,
    };

    let result = scan::run(&config);

    assert!(result.entries.is_empty());
}
