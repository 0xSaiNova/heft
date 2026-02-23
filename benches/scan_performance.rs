use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use heft::config::Config;
use heft::scan;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Fixture generator for realistic directory structures
mod fixtures {
    use super::*;

    /// Create a simple project with node_modules
    pub fn create_node_project(
        base: &Path,
        depth: usize,
        modules_per_level: usize,
    ) -> std::io::Result<()> {
        // Create package.json
        fs::write(
            base.join("package.json"),
            r#"{"name": "test-project", "version": "1.0.0"}"#,
        )?;

        // Create node_modules with depth
        create_node_modules(
            base.join("node_modules").as_path(),
            depth,
            modules_per_level,
        )?;
        Ok(())
    }

    fn create_node_modules(base: &Path, depth: usize, modules: usize) -> std::io::Result<()> {
        if depth == 0 {
            return Ok(());
        }

        fs::create_dir_all(base)?;

        for i in 0..modules {
            let module_dir = base.join(format!("module-{i}"));
            fs::create_dir_all(&module_dir)?;

            // Create package.json
            fs::write(
                module_dir.join("package.json"),
                format!(r#"{{"name": "module-{i}"}}"#),
            )?;

            // Create some dummy files
            fs::write(module_dir.join("index.js"), "module.exports = {};")?;
            fs::write(module_dir.join("README.md"), "# Module")?;

            // Recurse for nested node_modules
            if depth > 1 {
                create_node_modules(
                    module_dir.join("node_modules").as_path(),
                    depth - 1,
                    modules.saturating_sub(1),
                )?;
            }
        }
        Ok(())
    }

    /// Create a Rust project with target directory
    pub fn create_rust_project(base: &Path, with_target: bool) -> std::io::Result<()> {
        // Create Cargo.toml
        fs::write(
            base.join("Cargo.toml"),
            r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
        )?;

        // Create src directory
        fs::create_dir_all(base.join("src"))?;
        fs::write(base.join("src").join("main.rs"), "fn main() {}")?;

        // Optionally create target directory with artifacts
        if with_target {
            let target_dir = base.join("target").join("debug");
            fs::create_dir_all(&target_dir)?;

            // Create some fake build artifacts
            for i in 0..50 {
                fs::write(
                    target_dir.join(format!("artifact-{i}.rlib")),
                    vec![0u8; 1024 * 100], // 100KB files
                )?;
            }
        }

        Ok(())
    }

    /// Create a directory tree with many small files
    pub fn create_deep_tree(
        base: &Path,
        depth: usize,
        files_per_dir: usize,
    ) -> std::io::Result<()> {
        create_tree_recursive(base, depth, files_per_dir)
    }

    fn create_tree_recursive(
        base: &Path,
        depth: usize,
        files_per_dir: usize,
    ) -> std::io::Result<()> {
        if depth == 0 {
            return Ok(());
        }

        fs::create_dir_all(base)?;

        // Create files at this level
        for i in 0..files_per_dir {
            fs::write(base.join(format!("file-{i}.txt")), "test content")?;
        }

        // Create subdirectories
        for i in 0..3 {
            let subdir = base.join(format!("dir-{i}"));
            create_tree_recursive(&subdir, depth - 1, files_per_dir)?;
        }

        Ok(())
    }

    /// Create mixed cache directories (npm, cargo, etc)
    pub fn create_cache_dirs(base: &Path) -> std::io::Result<()> {
        // NPM cache
        let npm_cache = base.join(".npm");
        fs::create_dir_all(&npm_cache)?;
        for i in 0..100 {
            fs::write(
                npm_cache.join(format!("package-{i}.tgz")),
                vec![0u8; 1024 * 10], // 10KB
            )?;
        }

        // Cargo cache
        let cargo_cache = base.join(".cargo").join("registry");
        fs::create_dir_all(&cargo_cache)?;
        for i in 0..50 {
            fs::write(
                cargo_cache.join(format!("crate-{i}.crate")),
                vec![0u8; 1024 * 20], // 20KB
            )?;
        }

        Ok(())
    }
}

/// Helper to create config for benchmarking
fn create_bench_config(roots: Vec<PathBuf>) -> Config {
    Config {
        roots,
        timeout: std::time::Duration::from_secs(30),
        disabled_detectors: vec!["docker".to_string()], // Skip docker in benchmarks for consistency
        json_output: false,
        verbose: false,
        progressive: false,
        platform: heft::platform::detect(),
    }
}

/// Benchmark: Small directory scan (minimal overhead)
fn bench_small_scan(c: &mut Criterion) {
    c.bench_function("scan_small_directory", |b| {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal project
        fixtures::create_rust_project(temp_dir.path(), false).unwrap();

        let config = create_bench_config(vec![temp_dir.path().to_path_buf()]);

        b.iter(|| {
            let result = scan::run(black_box(&config));
            black_box(result);
        });
    });
}

/// Benchmark: Node.js project with moderate nesting
fn bench_node_modules_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("scan_node_modules");

    for depth in [2, 3, 4] {
        group.bench_with_input(BenchmarkId::new("depth", depth), &depth, |b, &depth| {
            let temp_dir = TempDir::new().unwrap();
            fixtures::create_node_project(temp_dir.path(), depth, 5).unwrap();
            let config = create_bench_config(vec![temp_dir.path().to_path_buf()]);

            b.iter(|| {
                let result = scan::run(black_box(&config));
                black_box(result);
            });
        });
    }

    group.finish();
}

/// Benchmark: Rust project with build artifacts
fn bench_rust_project_scan(c: &mut Criterion) {
    c.bench_function("scan_rust_with_target", |b| {
        let temp_dir = TempDir::new().unwrap();
        fixtures::create_rust_project(temp_dir.path(), true).unwrap();
        let config = create_bench_config(vec![temp_dir.path().to_path_buf()]);

        b.iter(|| {
            let result = scan::run(black_box(&config));
            black_box(result);
        });
    });
}

/// Benchmark: Deep directory tree (stress test filesystem traversal)
fn bench_deep_tree_scan(c: &mut Criterion) {
    c.bench_function("scan_deep_tree", |b| {
        let temp_dir = TempDir::new().unwrap();
        // Depth 4, 5 files per directory = ~400 files
        fixtures::create_deep_tree(temp_dir.path(), 4, 5).unwrap();
        let config = create_bench_config(vec![temp_dir.path().to_path_buf()]);

        b.iter(|| {
            let result = scan::run(black_box(&config));
            black_box(result);
        });
    });
}

/// Benchmark: Cache detection
fn bench_cache_scan(c: &mut Criterion) {
    c.bench_function("scan_cache_directories", |b| {
        let temp_dir = TempDir::new().unwrap();
        fixtures::create_cache_dirs(temp_dir.path()).unwrap();
        let config = create_bench_config(vec![temp_dir.path().to_path_buf()]);

        b.iter(|| {
            let result = scan::run(black_box(&config));
            black_box(result);
        });
    });
}

/// Benchmark: Memory usage validation
fn bench_memory_usage(c: &mut Criterion) {
    c.bench_function("scan_memory_tracking", |b| {
        let temp_dir = TempDir::new().unwrap();

        // Create a moderate-sized fixture
        fixtures::create_node_project(temp_dir.path(), 3, 5).unwrap();

        // Create subdirectory for rust project
        let rust_proj_dir = temp_dir.path().join("rust-proj");
        fs::create_dir(&rust_proj_dir).unwrap();
        fixtures::create_rust_project(&rust_proj_dir, true).unwrap();

        let config = create_bench_config(vec![temp_dir.path().to_path_buf()]);

        b.iter(|| {
            let result = scan::run(black_box(&config));

            // Validate memory tracking is working
            assert!(
                result.peak_memory_bytes.is_some(),
                "Memory tracking should be enabled"
            );
            assert!(
                !result.detector_memory.is_empty(),
                "Per-detector memory should be tracked"
            );

            black_box(result);
        });
    });
}

/// Benchmark: Timing accuracy
fn bench_timing_accuracy(c: &mut Criterion) {
    c.bench_function("scan_timing_accuracy", |b| {
        let temp_dir = TempDir::new().unwrap();
        fixtures::create_rust_project(temp_dir.path(), true).unwrap();
        let config = create_bench_config(vec![temp_dir.path().to_path_buf()]);

        b.iter(|| {
            let result = scan::run(black_box(&config));

            // Validate timing is captured
            assert!(result.duration_ms.is_some(), "Duration should be captured");
            assert!(
                !result.detector_timings.is_empty(),
                "Per-detector timing should be captured"
            );

            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_small_scan,
    bench_node_modules_scan,
    bench_rust_project_scan,
    bench_deep_tree_scan,
    bench_cache_scan,
    bench_memory_usage,
    bench_timing_accuracy,
);

criterion_main!(benches);
