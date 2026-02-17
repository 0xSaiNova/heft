# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Integer overflow in diff engine delta calculations - `size_bytes as i64` silently truncated large values, negating the result could panic in debug mode on i64::MIN. Now uses `i64::try_from` with saturating arithmetic throughout (#78)
- `make_key` in diff engine used `{:?}` debug format for snapshot lookup keys - any variant rename would silently break old snapshot matching. Now uses `as_str()` which is the stable DB representation (#78)
- `total_bytes` fold in `save_snapshot` used wrapping addition, now uses `saturating_add` (#78)
- pip cache path on Windows was using Linux path `~/.cache/pip`, corrected to `AppData/Local/pip/Cache` (#79)
- VSCode data path on Windows was using Linux path `~/.config/Code`, corrected to `AppData/Roaming/Code` (#79)
- pnpm store path was hardcoded to Linux path for all platforms, now platform-aware (macOS: `Library/pnpm/store`, Windows: `AppData/Local/pnpm/store`) (#79)
- Docker detector used `.output()` which blocks indefinitely if Docker Desktop is starting. Now uses spawn+poll with `config.timeout` (#80)
- Docker `available()` check spawned `docker --version` redundantly since `scan()` already handles not-installed gracefully. Removed the extra spawn (#80)
- `detect_docker_desktop_vm` called `platform::detect()` directly instead of using `config.platform`, making platform untestable (#80)
- `heft clean --yes --dry-run` silently executed deletion instead of erroring. Now rejected at parse time with `conflicts_with` (#81)
- `heft clean` ignored all scan configuration - `--roots`, `--timeout`, `--no-docker` had no effect. Clean now accepts and uses these flags (#81)
- `--category` on clean silently dropped unrecognized values, producing an empty filter with no feedback. Now uses `ValueEnum` so invalid values are rejected at parse time (#81)
- `docker rmi` in `delete_docker_object` was missing `--` before the object ID, allowing crafted IDs to be interpreted as flags (#81)
- Snapshot functions each opened their own SQLite connection. The diff command opened 4 connections for a single user action. Replaced with a `Store` struct that holds one connection per command (#82)
- SQLite foreign keys were declared but never enforced (`PRAGMA foreign_keys = ON` was missing). Entries now also cascade delete when their parent snapshot is deleted (#82)
- `load_snapshot_entries` failures in the report command were swallowed with `unwrap_or_default()`, silently showing an empty table on a corrupted database (#82)
- `snapshot.rs` lived outside the `store/` module it belongs to. Moved to `store/snapshot.rs` (#82)
- `get_source_last_modified` used `continue` to skip artifact directories but walkdir still descended into them, scanning thousands of files unnecessarily. Now uses `filter_entry` to prune traversal entirely (#83)
- `is_xcode_derived_data` walked all ancestors up to `/` calling `read_dir` on each. Now bounded to home directory with a hard cap of 10 levels (#83)

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
