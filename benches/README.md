# benchmarks

tracks performance and catches regressions.

## running

```bash
cargo bench                  # run all
cargo bench -- scan_small    # run one
cargo bench -- --test        # quick test
```

## what we test

**small scan** - baseline overhead, minimal project
**node modules** - deep nesting at 2/3/4 levels, simulates monorepos
**rust project** - target/ detection, build artifacts
**deep tree** - ~400 files, stresses filesystem traversal
**caches** - npm/cargo cache detection
**memory** - validates tracking works (needs #44)
**timing** - validates per-detector metrics (needs #39)

## metrics

criterion tracks time, throughput, mean/median/stddev automatically.
memory and timing benchmarks validate those features work correctly.

## ci

runs on PRs and main branch. results in `target/criterion/`, view reports at `report/index.html`.

## adding tests

1. add fixture to `fixtures` module
2. write bench function
3. add to `criterion_group!` macro

keep it deterministic - no network, no docker, use tempfile for cleanup.
