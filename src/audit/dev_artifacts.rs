//! Dev artifact classification with parent file guards.
//!
//! Replicates the detection logic from projects.rs and caches.rs as pure
//! path checks. Used by the audit category engine to correctly identify
//! build artifacts, package caches, and IDE data.

use std::path::Path;

use super::categories::AuditCategory;

/// Classify a directory as a dev artifact based on its name and parent context.
/// Returns None if the directory is not a known artifact pattern.
pub fn classify(path: &Path, dir_name: &str) -> Option<(AuditCategory, Option<String>)> {
    let parent = path.parent()?;

    match dir_name {
        "node_modules" => Some((AuditCategory::DevArtifacts, Some("node_modules".into()))),

        "target" if parent.join("Cargo.toml").exists() => {
            Some((AuditCategory::DevArtifacts, Some("Rust target".into())))
        }

        "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".tox" => {
            Some((AuditCategory::DevArtifacts, Some(dir_name.into())))
        }

        ".venv" | "venv" if has_python_project(parent) => {
            Some((AuditCategory::DevArtifacts, Some("Python venv".into())))
        }

        "vendor" if parent.join("go.mod").exists() => {
            Some((AuditCategory::DevArtifacts, Some("Go vendor".into())))
        }
        "vendor" if parent.join("composer.json").exists() => {
            Some((AuditCategory::DevArtifacts, Some("PHP vendor".into())))
        }

        ".gradle" if has_gradle_project(parent) => {
            Some((AuditCategory::DevArtifacts, Some("Gradle cache".into())))
        }
        "build" if has_gradle_project(parent) && is_gradle_build_dir(path) => {
            Some((AuditCategory::DevArtifacts, Some("Gradle build".into())))
        }

        "DerivedData" if is_xcode_derived_data(path) => Some((
            AuditCategory::DevArtifacts,
            Some("Xcode DerivedData".into()),
        )),

        "bin" | "obj" if has_dotnet_project(parent) => {
            Some((AuditCategory::DevArtifacts, Some(".NET build".into())))
        }

        _ => None,
    }
}

/// Classify known developer tool cache paths relative to home directory.
pub fn classify_cache(rel_str: &str) -> Option<(AuditCategory, Option<String>)> {
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
