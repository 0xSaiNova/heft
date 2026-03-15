//! Running process signal for activity detection.
//!
//! Checks if any running process has a working directory under a given project root.
//! Linux: reads /proc/[pid]/cwd symlinks. Other platforms: returns empty (best effort).

use std::collections::HashMap;
use std::path::PathBuf;

/// Known dev process names that strongly indicate active development.
const DEV_PROCESSES: &[&str] = &[
    "node", "cargo", "rustc", "gradle", "python", "python3",
    "webpack", "vite", "next", "dotnet", "go", "flask", "uvicorn",
];

/// Check if any running process has its cwd under one of the given roots.
/// Returns a map from root path to a human readable reason string.
/// Silently returns empty on any failure (best effort signal).
pub fn active_roots(roots: &[PathBuf]) -> HashMap<PathBuf, String> {
    if roots.is_empty() {
        return HashMap::new();
    }

    #[cfg(target_os = "linux")]
    {
        scan_proc_cwd(roots)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = roots;
        HashMap::new()
    }
}

#[cfg(target_os = "linux")]
fn scan_proc_cwd(roots: &[PathBuf]) -> HashMap<PathBuf, String> {
    let mut result = HashMap::new();

    let proc_entries = match std::fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return result,
    };

    for entry in proc_entries.flatten() {
        let name = entry.file_name();
        let pid_str = match name.to_str() {
            Some(s) if s.chars().all(|c| c.is_ascii_digit()) => s.to_string(),
            _ => continue,
        };

        // read the cwd symlink
        let cwd_path = format!("/proc/{pid_str}/cwd");
        let cwd = match std::fs::read_link(&cwd_path) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // check if this cwd is under any of our roots
        for root in roots {
            if cwd.starts_with(root) {
                // already found this root, skip
                if result.contains_key(root) {
                    continue;
                }

                // try to get the process name for a better reason string
                let comm_path = format!("/proc/{pid_str}/comm");
                let process_name = std::fs::read_to_string(&comm_path)
                    .ok()
                    .map(|s| s.trim().to_string());

                let reason = match &process_name {
                    Some(name) if DEV_PROCESSES.contains(&name.as_str()) => {
                        format!("{name} running (pid {pid_str})")
                    }
                    Some(name) => format!("process {name} (pid {pid_str})")                    ,
                    None => format!("process (pid {pid_str})"),
                };

                result.insert(root.clone(), reason);
            }
        }

        // if we found all roots, stop early
        if result.len() == roots.len() {
            break;
        }
    }

    result
}
