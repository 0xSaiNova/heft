# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-02-15

### Added
- SQLite snapshot storage - every scan auto-saves to `~/.local/share/heft/heft.db`
- `heft report --list` to view all saved snapshots
- `heft report --id N` to replay a specific snapshot
- `heft diff` to compare the two most recent snapshots
- `heft diff --from N --to M` to compare specific snapshots
- Diff engine tracks four change types: grew, shrank, new, gone
- Per-detector timing in verbose mode and JSON output
- Progressive output with `--progressive` flag
- Memory monitoring with peak RSS and per-detector deltas
- Benchmarking suite with Criterion

### Changed
- Snapshot duration type from u128 to u64 (matches SQLite storage)
- Diff output uses plain text markers instead of unicode emojis
- Category labels in diff output use human-readable names

### Fixed
- Integer truncation when storing u64 values in SQLite (now uses try_from with safe fallback)
- Negative values from SQLite reads now clamped to zero
- `heft report --id abc` no longer panics (proper error message and exit)
- `heft diff` no longer panics on database errors (proper error handling throughout)
- Removed dead `--last` flag from diff command
- Removed conflicting stub code in store module
- Report command no longer loads snapshot entries twice
- `--from` and `--to` flags now validated to be used together

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
