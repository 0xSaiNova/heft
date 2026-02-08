# heft

A disk space auditor for developers that shows where your space went and how much you can safely reclaim.

## The Problem

Developer workstations accumulate disk bloat silently. Docker images pile up, node_modules multiply across abandoned projects, cargo targets grow unbounded, package manager caches swell in hidden directories. A typical polyglot developer can lose 40-80 GB to reclaimable junk without realizing it.

## Status

Early development. Not yet usable. The project skeleton exists but core functionality is being implemented.

## Building

```
cargo build
cargo test
```

## Roadmap

- [ ] v0.1 — Project and cache detectors, TUI table output
- [ ] v0.2 — Docker detector
- [ ] v0.3 — Cleanup engine with dry run
- [ ] v0.4 — SQLite snapshots and diff
- [ ] v0.5 — JSON output, config file, polish
- [ ] v1.0 — Stable release, cross platform, published

## License

MIT
