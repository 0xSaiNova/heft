# Architecture

This document describes heft's internal architecture.

## Module Overview

```
src/
  main.rs          entry point, cli dispatch
  cli.rs           clap argument definitions
  config.rs        runtime configuration
  platform.rs      os detection, path resolution
  scan/
    mod.rs         orchestrator
    detector.rs    trait and core types
    projects.rs    build artifact detector
    caches.rs      package cache detector
    docker.rs      container storage detector
  report/
    mod.rs         output formatting
    table.rs       terminal table renderer
    json.rs        json serializer
  clean/
    mod.rs         cleanup engine
  store/
    mod.rs         module declarations
    snapshot.rs    sqlite persistence (Store struct)
    diff.rs        snapshot comparison engine
```

## Core Types

The `Detector` trait is the central abstraction. Each detector implements:
- `name()` - identifier for diagnostics
- `available()` - platform check
- `scan()` - returns `DetectorResult` containing `BloatEntry` items

`BloatEntry` is the universal unit of detected bloat with category, location, size, and reclaimable bytes.

## Data Flow

1. CLI parses args into `ScanArgs` or `CleanArgs`
2. `Config` constructed from args and platform detection
3. Orchestrator runs each available detector
4. Results merged into `ScanResult`
5. Reporter formats output for terminal or JSON
6. `Store::open()` opens a single SQLite connection and saves the snapshot

## Snapshot Storage

`Store` in `store/snapshot.rs` owns one SQLite connection for the lifetime of a command. All snapshot operations go through it - no function opens its own connection. Foreign keys are enforced via `PRAGMA foreign_keys = ON` on open, and entries cascade delete when their parent snapshot is removed.
