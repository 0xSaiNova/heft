# Stress Testing

Notes on stress testing heft against large filesystems.

## Test Scenarios

- Large home directory (500k+ files)
- Deep nesting (node_modules in monorepos)
- Many small files vs few large files
- Slow filesystems (network mounts, spinning disks)
- Docker with hundreds of images

## Metrics to Track

- Wall clock time for full scan
- Memory usage peak
- CPU utilization
- Time per detector

## Known Bottlenecks

- Filesystem traversal dominates runtime
- Docker daemon queries can timeout
- Large cargo registries are slow to stat
