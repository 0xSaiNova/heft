//! Audit category system and classification engine.
//!
//! Assigns every file/directory to a two-level category hierarchy.
//! Priority order: dev artifact patterns > path match > custom rules > extension > Other

use std::path::Path;

use serde::Serialize;

use crate::scan::detector::ARTIFACT_DIR_NAMES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum AuditCategory {
    DevArtifacts,
    Media,
    Documents,
    Downloads,
    Logs,
    ApplicationData,
    VersionControl,
    System,
    TrashTemp,
    Other,
}

impl AuditCategory {
    pub fn label(&self) -> &'static str {
        match self {
            AuditCategory::DevArtifacts => "Dev Artifacts",
            AuditCategory::Media => "Media",
            AuditCategory::Documents => "Documents",
            AuditCategory::Downloads => "Downloads",
            AuditCategory::Logs => "Logs",
            AuditCategory::ApplicationData => "Application Data",
            AuditCategory::VersionControl => "Version Control",
            AuditCategory::System => "System",
            AuditCategory::TrashTemp => "Trash / Temp",
            AuditCategory::Other => "Other",
        }
    }

    pub fn sort_order(&self) -> u8 {
        match self {
            AuditCategory::DevArtifacts => 0,
            AuditCategory::Media => 1,
            AuditCategory::Documents => 2,
            AuditCategory::Downloads => 3,
            AuditCategory::ApplicationData => 4,
            AuditCategory::Logs => 5,
            AuditCategory::VersionControl => 6,
            AuditCategory::System => 7,
            AuditCategory::TrashTemp => 8,
            AuditCategory::Other => 9,
        }
    }
}

/// User-defined classification rule from config.toml
pub struct CustomRule {
    pub path_contains: Option<String>,
    pub extension: Option<Vec<String>>,
    pub category: AuditCategory,
    pub subcategory: Option<String>,
}

// extension lists for classification
const VIDEO_EXT: &[&str] = &["mp4", "mov", "mkv", "avi", "webm", "wmv", "flv"];
const AUDIO_EXT: &[&str] = &["mp3", "flac", "wav", "aac", "ogg", "m4a", "wma"];
const IMAGE_EXT: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "raw", "cr2", "heic", "psd", "tiff", "bmp", "svg", "ico",
];
const DOC_EXT: &[&str] = &[
    "pdf", "docx", "doc", "xlsx", "xls", "pptx", "ppt", "md", "txt", "csv", "epub", "pages",
    "numbers", "key", "odt", "ods", "odp", "rtf",
];

/// Classify a path into a category and optional subcategory.
/// Called inline during the audit walk for each entry.
pub fn classify_path(
    path: &Path,
    dir_name: &str,
    extension: Option<&str>,
    home: Option<&Path>,
    custom_rules: &[CustomRule],
) -> (AuditCategory, Option<String>) {
    // 1. dev artifact pattern match (highest priority)
    if ARTIFACT_DIR_NAMES.contains(&dir_name) {
        return (AuditCategory::DevArtifacts, Some(dir_name.to_string()));
    }

    // 2. path-based classification
    if let Some(result) = classify_by_path(path, home) {
        return result;
    }

    // 3. custom rules
    for rule in custom_rules {
        if let Some(ref pattern) = rule.path_contains {
            let path_str = path.to_string_lossy();
            if path_str.contains(pattern.as_str()) {
                return (rule.category, rule.subcategory.clone());
            }
        }
        if let Some(ref exts) = rule.extension {
            if let Some(ext) = extension {
                if exts.iter().any(|e| e.trim_start_matches('.') == ext) {
                    return (rule.category, rule.subcategory.clone());
                }
            }
        }
    }

    // 4. extension-based classification
    if let Some(ext) = extension {
        let ext_lower = ext.to_lowercase();
        let ext_ref = ext_lower.as_str();

        if VIDEO_EXT.contains(&ext_ref) {
            return (AuditCategory::Media, Some("Video".to_string()));
        }
        if AUDIO_EXT.contains(&ext_ref) {
            return (AuditCategory::Media, Some("Audio".to_string()));
        }
        if IMAGE_EXT.contains(&ext_ref) {
            return (AuditCategory::Media, Some("Images".to_string()));
        }
        if DOC_EXT.contains(&ext_ref) {
            return (AuditCategory::Documents, None);
        }
        if ext_ref == "log" {
            return (AuditCategory::Logs, None);
        }
    }

    // 5. default
    (AuditCategory::Other, None)
}

fn classify_by_path(path: &Path, home: Option<&Path>) -> Option<(AuditCategory, Option<String>)> {
    let path_str = path.to_string_lossy();

    // .git/objects -> version control
    if path_str.contains(".git/objects") || path_str.contains(".git\\objects") {
        return Some((AuditCategory::VersionControl, None));
    }

    // system paths (Unix)
    if path.starts_with("/usr")
        || path.starts_with("/opt")
        || path.starts_with("/Library")
        || path.starts_with("/System")
    {
        return Some((AuditCategory::System, None));
    }
    if path.starts_with("/var") {
        if path.starts_with("/var/log") {
            return Some((AuditCategory::Logs, Some("system".to_string())));
        }
        return Some((AuditCategory::System, None));
    }

    // tmp
    if path.starts_with("/tmp") {
        return Some((AuditCategory::TrashTemp, None));
    }

    // home-relative paths
    if let Some(home) = home {
        if let Ok(rel) = path.strip_prefix(home) {
            let rel_str = rel.to_string_lossy();

            // downloads
            if rel_str.starts_with("Downloads") {
                return Some((AuditCategory::Downloads, None));
            }

            // trash
            if rel_str.starts_with(".Trash") || rel_str.starts_with(".local/share/Trash") {
                return Some((AuditCategory::TrashTemp, None));
            }

            // logs
            if rel_str.starts_with("Library/Logs") {
                return Some((AuditCategory::Logs, Some("app".to_string())));
            }

            // application data
            if rel_str.starts_with("Library/Application Support")
                || rel_str.starts_with(".local/share")
                || rel_str.starts_with(".config")
            {
                return Some((AuditCategory::ApplicationData, None));
            }

            // cache (not already caught by dev artifact patterns)
            if rel_str.starts_with(".cache") || rel_str.starts_with("Library/Caches") {
                return Some((AuditCategory::ApplicationData, Some("Cache".to_string())));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_node_modules_as_dev_artifacts() {
        let (cat, sub) = classify_path(
            &PathBuf::from("/home/user/project/node_modules"),
            "node_modules",
            None,
            Some(&PathBuf::from("/home/user")),
            &[],
        );
        assert_eq!(cat, AuditCategory::DevArtifacts);
        assert_eq!(sub.as_deref(), Some("node_modules"));
    }

    #[test]
    fn classifies_mp4_as_media() {
        let (cat, sub) = classify_path(
            &PathBuf::from("/home/user/video.mp4"),
            "video.mp4",
            Some("mp4"),
            Some(&PathBuf::from("/home/user")),
            &[],
        );
        assert_eq!(cat, AuditCategory::Media);
        assert_eq!(sub.as_deref(), Some("Video"));
    }

    #[test]
    fn classifies_downloads_by_path() {
        let home = PathBuf::from("/home/user");
        let (cat, _) = classify_path(
            &PathBuf::from("/home/user/Downloads/file.mp4"),
            "file.mp4",
            Some("mp4"),
            Some(&home),
            &[],
        );
        // path match takes priority over extension
        assert_eq!(cat, AuditCategory::Downloads);
    }

    #[test]
    fn classifies_var_log_as_logs() {
        let (cat, _) = classify_path(&PathBuf::from("/var/log/syslog"), "syslog", None, None, &[]);
        assert_eq!(cat, AuditCategory::Logs);
    }

    #[test]
    fn custom_rule_overrides_extension() {
        let rules = vec![CustomRule {
            path_contains: Some("recordings".to_string()),
            extension: None,
            category: AuditCategory::Media,
            subcategory: Some("Screen Recordings".to_string()),
        }];

        let (cat, sub) = classify_path(
            &PathBuf::from("/home/user/recordings/meeting.txt"),
            "meeting.txt",
            Some("txt"),
            Some(&PathBuf::from("/home/user")),
            &rules,
        );
        assert_eq!(cat, AuditCategory::Media);
        assert_eq!(sub.as_deref(), Some("Screen Recordings"));
    }

    #[test]
    fn unknown_file_is_other() {
        let (cat, _) = classify_path(
            &PathBuf::from("/home/user/random.xyz"),
            "random.xyz",
            Some("xyz"),
            Some(&PathBuf::from("/home/user")),
            &[],
        );
        assert_eq!(cat, AuditCategory::Other);
    }
}
