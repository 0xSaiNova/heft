# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-02-12

### Added
- Interactive cleanup mode as the new default - prompts per category before deletion
- `--dry-run` flag for non-interactive preview mode
- Path validation before deletion (absolute paths only, must be under home or temp)
- Stdin/stdout error handling with graceful fallback in interactive mode
- Category sorting for consistent display order
- Manifest file size check (1MB limit) to prevent OOM on malicious files

### Changed
- Default cleanup behavior from dry-run to interactive mode
- Cleanup modes now: interactive (default), `--dry-run` (preview), `--yes` (execute all)
- Duration tracking changed from u64 to u128 to prevent overflow
- Windows yarn cache path corrected to use proper AppData path structure

### Fixed
- Path validation no longer follows symlinks (security consistency)
- Windows yarn cache path now uses correct separators (AppData/Local/Yarn/Cache)
- Zombie process leak in homebrew timeout (now properly reaps child process)
- TOML parser breaking on `[dependencies]` section after `[package]`
- Integer overflow in directory size calculation (now uses checked_add and caps at u64::MAX)
- JSON error messages not escaping quotes (now uses serde_json::json! macro)
- Config timeout field unused (now applied to homebrew cache detection)
- Code quality: replaced unwrap() calls with proper error handling
- Code quality: applied clippy suggestions for cleaner code

## [0.2.0] - 2026-02-09

### Added
- Docker detection and cleanup support
- Docker Desktop VM detection for macOS and Windows
- Support for all docker object types (images, containers, volumes, build cache)

### Fixed
- Critical security vulnerabilities in cleanup (symlink attacks, TOCTOU)
- Detection accuracy bugs (false positives, swallowed errors)

## [0.1.0] - Initial Release

### Added
- Project artifact detection (node_modules, target, build directories)
- Package manager cache detection (npm, yarn, pip, cargo, etc)
- IDE data detection (VSCode)
- Scan and cleanup commands
- JSON output support
- Verbose logging mode
