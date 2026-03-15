//! Audit category system and classification engine.
//!
//! Assigns every file/directory to a two-level category hierarchy.
//! Priority order: dev artifact patterns > path match > custom rules >
//! parent context > extension > Other

use std::path::Path;

use serde::Serialize;

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
    // 1. dev artifact pattern match with parent file guards
    if let Some(result) = classify_dev_artifact(path, dir_name) {
        return result;
    }

    // 2. path-based classification (caches, known paths, system dirs)
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

    // 4. parent context (app data subdirectories inherit from parent)
    if let Some(result) = classify_by_parent_context(path, home) {
        return result;
    }

    // 5. extension-based classification
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

    // 6. default
    (AuditCategory::Other, None)
}

/// Classify directories that are known build artifacts with parent file guards.
/// Replicates the detection logic from projects.rs detect_artifact() as pure
/// path checks without importing detector modules.
fn classify_dev_artifact(path: &Path, dir_name: &str) -> Option<(AuditCategory, Option<String>)> {
    let parent = path.parent()?;

    match dir_name {
        // node_modules: always a dev artifact (very low false positive risk)
        "node_modules" => Some((AuditCategory::DevArtifacts, Some("node_modules".into()))),

        // target: only if parent has Cargo.toml (avoids ~/Documents/target/ false positive)
        "target" if parent.join("Cargo.toml").exists() => {
            Some((AuditCategory::DevArtifacts, Some("Rust target".into())))
        }

        // python caches: low false positive risk, no guard needed
        "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".tox" => {
            Some((AuditCategory::DevArtifacts, Some(dir_name.into())))
        }

        // venv: only with python project markers
        ".venv" | "venv" if has_python_project(parent) => {
            Some((AuditCategory::DevArtifacts, Some("Python venv".into())))
        }

        // vendor: go or php
        "vendor" if parent.join("go.mod").exists() => {
            Some((AuditCategory::DevArtifacts, Some("Go vendor".into())))
        }
        "vendor" if parent.join("composer.json").exists() => {
            Some((AuditCategory::DevArtifacts, Some("PHP vendor".into())))
        }

        // gradle
        ".gradle" if has_gradle_project(parent) => {
            Some((AuditCategory::DevArtifacts, Some("Gradle cache".into())))
        }
        "build" if has_gradle_project(parent) && is_gradle_build_dir(path) => {
            Some((AuditCategory::DevArtifacts, Some("Gradle build".into())))
        }

        // xcode DerivedData: check if under ~/Library/Developer/Xcode/ or has xcode markers
        "DerivedData" if is_xcode_derived_data(path) => Some((
            AuditCategory::DevArtifacts,
            Some("Xcode DerivedData".into()),
        )),

        // .NET bin/obj: only with project files
        "bin" | "obj" if has_dotnet_project(parent) => {
            Some((AuditCategory::DevArtifacts, Some(".NET build".into())))
        }

        _ => None,
    }
}

fn has_python_project(dir: &Path) -> bool {
    dir.join("requirements.txt").exists()
        || dir.join("setup.py").exists()
        || dir.join("pyproject.toml").exists()
        || dir.join("setup.cfg").exists()
}

fn has_gradle_project(dir: &Path) -> bool {
    dir.join("build.gradle").exists() || dir.join("build.gradle.kts").exists()
}

fn is_gradle_build_dir(path: &Path) -> bool {
    path.join("classes").exists()
        || path.join("libs").exists()
        || path.join("tmp").exists()
        || path.join("generated").exists()
        || path.join("intermediates").exists()
}

fn is_xcode_derived_data(path: &Path) -> bool {
    // check standard xcode cache location
    let in_xcode_cache = path.ancestors().any(|ancestor| {
        if let (Some(name), Some(parent)) = (ancestor.file_name(), ancestor.parent()) {
            name.to_str() == Some("Xcode")
                && parent
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s == "Developer")
                    .unwrap_or(false)
        } else {
            false
        }
    });

    let has_xcode_markers = path.join("Build").exists()
        || path.join("Logs").exists()
        || path.join("ModuleCache").exists()
        || path.join("info.plist").exists();

    in_xcode_cache || has_xcode_markers
}

fn has_dotnet_project(dir: &Path) -> bool {
    if dir.join("Directory.Build.props").exists()
        || dir.join("packages.config").exists()
        || dir.join("NuGet.Config").exists()
    {
        return true;
    }
    std::fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                let name = e.file_name();
                let s = name.to_str().unwrap_or("");
                s.ends_with(".csproj")
                    || s.ends_with(".fsproj")
                    || s.ends_with(".vbproj")
                    || s.ends_with(".sln")
            })
        })
        .unwrap_or(false)
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
            return Some((AuditCategory::Logs, Some("system".into())));
        }
        return Some((AuditCategory::System, None));
    }

    // tmp
    if path.starts_with("/tmp") {
        return Some((AuditCategory::TrashTemp, None));
    }

    // home-relative paths
    let home = home?;
    let rel = path.strip_prefix(home).ok()?;
    let rel_str = rel.to_string_lossy();

    // dev caches (checked before general app data so they get DevArtifacts, not ApplicationData)
    if let Some(result) = classify_dev_cache(&rel_str) {
        return Some(result);
    }

    // docker desktop
    if rel_str.starts_with("Library/Containers/com.docker.docker") {
        return Some((AuditCategory::DevArtifacts, Some("Docker Desktop".into())));
    }

    // xcode derived data (path based, separate from the dir name guard above)
    if rel_str.starts_with("Library/Developer/Xcode/DerivedData") {
        return Some((
            AuditCategory::DevArtifacts,
            Some("Xcode DerivedData".into()),
        ));
    }

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
        return Some((AuditCategory::Logs, Some("app".into())));
    }

    // application data: extract app name from next path component
    let app_data_prefixes: &[&str] = &["Library/Application Support/", ".local/share/", ".config/"];
    for prefix in app_data_prefixes {
        if let Some(rest) = rel_str.strip_prefix(prefix) {
            let app_name = rest.split('/').next().unwrap_or("");
            let sub = if app_name.is_empty() {
                None
            } else {
                Some(app_name.to_string())
            };
            return Some((AuditCategory::ApplicationData, sub));
        }
    }
    // bare directory matches (no trailing slash)
    if rel_str == "Library/Application Support" || rel_str == ".local/share" || rel_str == ".config"
    {
        return Some((AuditCategory::ApplicationData, None));
    }

    // cache (general, not already caught by dev cache patterns)
    if rel_str.starts_with(".cache") || rel_str.starts_with("Library/Caches") {
        return Some((AuditCategory::ApplicationData, Some("Cache".into())));
    }

    None
}

/// Classify known developer tool cache paths relative to home.
fn classify_dev_cache(rel_str: &str) -> Option<(AuditCategory, Option<String>)> {
    let checks: &[(&str, &str)] = &[
        (".npm", "npm cache"),
        (".cache/yarn", "yarn cache"),
        ("Library/Caches/Yarn", "yarn cache"),
        (".local/share/pnpm/store", "pnpm store"),
        ("Library/pnpm/store", "pnpm store"),
        (".cache/pip", "pip cache"),
        ("Library/Caches/pip", "pip cache"),
        (".cargo/registry", "cargo registry"),
        (".cargo/git", "cargo git"),
        ("go/pkg/mod", "go module cache"),
        (".gradle/caches", "gradle cache"),
        (".m2/repository", "maven cache"),
        (".nuget/packages", "nuget cache"),
        (".android/avd", "android AVD"),
        (".android/cache", "android SDK cache"),
        ("Library/Android/sdk", "android SDK"),
        ("Android/Sdk", "android SDK"),
        (".config/Code", "vscode data"),
        ("Library/Application Support/Code", "vscode data"),
    ];

    for (prefix, subcategory) in checks {
        if rel_str.starts_with(prefix) {
            return Some((AuditCategory::DevArtifacts, Some((*subcategory).into())));
        }
    }

    None
}

/// Classify files inside known application data directories by extracting
/// the app name from the path hierarchy.
fn classify_by_parent_context(
    path: &Path,
    home: Option<&Path>,
) -> Option<(AuditCategory, Option<String>)> {
    let home = home?;
    let rel = path.strip_prefix(home).ok()?;
    let components: Vec<_> = rel.components().collect();

    // ~/Library/Application Support/{app}/...
    // ~/.local/share/{app}/...
    // ~/.config/{app}/...
    let app_data_prefixes: &[&[&str]] = &[
        &["Library", "Application Support"],
        &[".local", "share"],
        &[".config"],
    ];

    for prefix in app_data_prefixes {
        if components.len() > prefix.len() {
            let matches = components
                .iter()
                .zip(prefix.iter())
                .all(|(c, p)| c.as_os_str().to_str() == Some(p));
            if matches {
                let app_name = components[prefix.len()]
                    .as_os_str()
                    .to_string_lossy()
                    .to_string();
                return Some((AuditCategory::ApplicationData, Some(app_name)));
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
    fn target_without_cargo_toml_is_not_dev_artifact() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("Documents").join("target");
        std::fs::create_dir_all(&target).unwrap();
        // no Cargo.toml in parent — should NOT be DevArtifacts

        let (cat, _) = classify_path(&target, "target", None, Some(tmp.path()), &[]);
        assert_ne!(cat, AuditCategory::DevArtifacts);
    }

    #[test]
    fn target_with_cargo_toml_is_dev_artifact() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("myproject");
        std::fs::create_dir_all(project.join("target")).unwrap();
        std::fs::write(project.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let (cat, sub) = classify_path(
            &project.join("target"),
            "target",
            None,
            Some(tmp.path()),
            &[],
        );
        assert_eq!(cat, AuditCategory::DevArtifacts);
        assert_eq!(sub.as_deref(), Some("Rust target"));
    }

    #[test]
    fn cargo_registry_is_dev_artifact() {
        let home = PathBuf::from("/home/user");
        let (cat, sub) = classify_path(
            &PathBuf::from("/home/user/.cargo/registry/crates"),
            "crates",
            None,
            Some(&home),
            &[],
        );
        assert_eq!(cat, AuditCategory::DevArtifacts);
        assert_eq!(sub.as_deref(), Some("cargo registry"));
    }

    #[test]
    fn npm_cache_is_dev_artifact() {
        let home = PathBuf::from("/home/user");
        let (cat, sub) = classify_path(
            &PathBuf::from("/home/user/.npm/_cacache"),
            "_cacache",
            None,
            Some(&home),
            &[],
        );
        assert_eq!(cat, AuditCategory::DevArtifacts);
        assert_eq!(sub.as_deref(), Some("npm cache"));
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

    #[test]
    fn parent_context_classifies_slack_log_as_app_data() {
        let home = PathBuf::from("/home/user");
        let (cat, sub) = classify_path(
            &PathBuf::from("/home/user/.config/Slack/logs/app.log"),
            "app.log",
            Some("log"),
            Some(&home),
            &[],
        );
        assert_eq!(cat, AuditCategory::ApplicationData);
        assert_eq!(sub.as_deref(), Some("Slack"));
    }
}
