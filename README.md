# heft

**you probably have 20+ GB of garbage on your machine right now.**

build artifacts, docker layers, package caches, old node_modules from projects you haven't touched in a year. heft finds all of it in ~3 seconds and lets you clean it up without guessing.

**with cargo:**

```bash
git clone https://github.com/0xSaiNova/heft.git
cd heft && cargo install --path .
```

**without cargo** — install Rust first (takes ~1 min), then run the above:

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

```powershell
# Windows — run in PowerShell
winget install Rustlang.Rustup
```

Restart your terminal after installing Rust, then run the `cargo install` line above.

---

## run it

```
$ heft scan

Category            Name                          Size       Reclaimable  Age
─────────────────────────────────────────────────────────────────────────────────
Project Artifacts   node_modules (old-project)     2.1 GB     2.1 GB      120d
                    target (rust-experiments)      890 MB     890 MB       45d
                    .venv (ml-pipeline)            340 MB     340 MB       90d

Package Cache       npm cache                      3.2 GB     3.2 GB
                    cargo registry                 1.8 GB     1.8 GB
                    pip cache                      640 MB     640 MB

Container Data      docker images                  7.3 GB     4.1 GB
                    docker build cache             2.8 GB     2.8 GB

Total: 20.3 GB found, 16.8 GB reclaimable
Scan completed in 3.12s (peak memory: 27.4 MB)
```

## clean it up

```
$ heft clean

Project Artifacts: 3.3 GB (3 items)
  Delete? [y/n]: y

Package Cache: 5.6 GB (3 items)
  Delete? [y/n]: n

Container Data: 7.8 GB (3 items)
  Delete? [y/n]: y

Freed 11.1 GB
```

interactive by default — you approve each category before anything gets deleted. no surprises.

```bash
heft clean --dry-run                        # see exactly what would go
heft clean --yes                            # skip prompts, delete everything
heft clean --category project-artifacts     # only clean one category
heft clean --roots ~/code --no-docker       # control what gets scanned first
```

## watch your disk over time

every scan saves automatically. no setup.

```
$ heft diff

Package Cache:
  [+] npm cache grew 1.2 GB -> 1.8 GB (+600 MB)
  [-] cargo registry shrank 2.1 GB -> 1.4 GB (-700 MB)

Project Artifacts:
  [new] node_modules (new-project) appeared (450 MB)
  [gone] target (old-experiment) cleaned up (was 890 MB)

Net change: 540 MB freed
```

```bash
heft report --list          # see all saved snapshots
heft report --id 3          # replay any past scan
heft diff --from 1 --to 5   # compare any two
```

## what it finds

| | |
|---|---|
| **project artifacts** | `node_modules`, `target`, `.venv`, `bin`/`obj` (.NET), gradle/maven builds, Xcode DerivedData |
| **package caches** | npm, yarn, pnpm, pip, cargo, homebrew, go modules, maven, gradle, NuGet |
| **docker** | images, containers, volumes, build cache, Desktop VM disk files, WSL2 virtual disks |
| **IDE data** | VSCode, Android AVD emulator images, Android SDK |

## config file

persistent settings in `~/.config/heft/config.toml` — CLI flags always override:

```toml
[scan]
roots = ["/home/you/code"]
timeout = 60
verbose = true

[detectors]
docker = false   # skip docker entirely
xcode = false    # skip xcode on this machine
```

## scripting

```bash
heft scan --json | jq '.entries[] | select(.size_bytes > 1073741824)'
heft scan --progressive          # stream results as each detector finishes
heft scan --verbose              # show per-detector timing and diagnostics
heft scan --disable docker,xcode # skip specific detectors for one run
```

## safety

never touches source files. validates every path before deletion (must be absolute, under home). refuses to follow symlinks. interactive by default.

---

MIT license
