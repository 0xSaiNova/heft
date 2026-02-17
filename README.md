# heft

**Your disk is full. Again.**

heft scans your home directory, finds every build artifact, package cache, and docker object eating your space, and lets you clean it up safely. takes about 3 seconds.

## install

```bash
git clone https://github.com/0xSaiNova/heft.git
cd heft && cargo build --release
cp target/release/heft ~/.local/bin/
```

## scan

```bash
$ heft scan
```
```
Category            Name                          Size       Reclaimable  Age
─────────────────────────────────────────────────────────────────────────────────
Project Artifacts   node_modules (old-project)     2.1 GB     2.1 GB      120d
                    target (rust-experiments)      890 MB     890 MB       45d
                    .venv (ml-pipeline)            340 MB     340 MB       90d

Package Cache       npm cache                      3.2 GB     3.2 GB
                    cargo registry                 1.8 GB     1.8 GB
                    pip cache                      640 MB     640 MB

Container Data      Docker Images                  7.3 GB     4.1 GB
                    Docker Build Cache             2.8 GB     2.8 GB
                    Docker Volumes                 1.2 GB     900 MB

Total: 20.3 GB found, 16.8 GB reclaimable across 9 items
Scan completed in 3.12s (peak memory: 27.4 MB)
```

## clean

```bash
$ heft clean
```
```
Project Artifacts: 3.3 GB (3 items)
  Delete? [y/n]: y

Package Cache: 5.6 GB (3 items)
  Delete? [y/n]: n

Container Data: 7.8 GB (3 items)
  Delete? [y/n]: y

Freed 11.1 GB
```

you pick what goes. `--dry-run` to preview, `--yes` to skip prompts, `--category container-data` to target specific stuff.

## track changes over time

every scan auto-saves to a local sqlite database. no config needed.

```bash
$ heft report --list
```
```
ID     Date                 Total        Reclaimable
────────────────────────────────────────────────────
5      2026-02-15 09:15     14.2 GB      11.3 GB
4      2026-02-12 18:30     18.7 GB      15.1 GB
3      2026-02-10 14:00     20.3 GB      16.8 GB
```

```bash
$ heft diff
```
```
Comparing snapshots:
  From: #4 (2026-02-12 18:30:00)
  To:   #5 (2026-02-15 09:15:00)

Package Cache:
  [+] npm cache grew 1.2 GB -> 1.8 GB (+600 MB)
  [-] cargo registry shrank 2.1 GB -> 1.4 GB (-700 MB)

Project Artifacts:
  [new] node_modules (new-project) appeared (450 MB)
  [gone] target (old-experiment) cleaned up (was 890 MB)

Net change: 540 MB freed
```

`heft diff --from 1 --to 5` to compare any two snapshots.

## what it detects

| Category | What it finds |
|----------|---------------|
| Project artifacts | `node_modules/`, `target/`, `.venv/`, gradle/maven builds, DerivedData |
| Package caches | npm, yarn, pnpm, cargo, pip, homebrew, go, maven, gradle |
| Docker | images, containers, volumes, build cache, Desktop VM files |
| IDE data | VSCode extensions and caches, language servers |

## scripting

```bash
heft scan --json | jq '.entries[] | select(.size_bytes > 1000000000)'
heft scan --verbose              # per-detector timing and diagnostics
heft scan --progressive          # results as each detector finishes
heft scan --roots ~/code,~/work  # scan specific directories
```

## safety

- never touches source code or project files
- validates paths before deletion (absolute, under home/temp only)
- refuses to follow symlinks
- interactive mode by default

## performance

- ~3 second full home directory scan
- ~27 MB peak memory
- ~58ms on synthetic benchmarks (criterion)
- single binary, no runtime dependencies

## license

MIT
