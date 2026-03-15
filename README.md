# heft

**You probably have 20+ GB of build junk on your machine right now.**

Old node_modules, cargo targets, docker layers, package caches. heft finds all of it in seconds, ranks it by how stale it is, and lets you clean it up safely.

## Install

```bash
git clone https://github.com/0xSaiNova/heft.git
cd heft && cargo install --path .
```

Need Rust first? Takes about a minute.

```bash
# macOS and Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

```powershell
# Windows (PowerShell)
winget install Rustlang.Rustup
```

Restart your terminal after installing Rust, then run the cargo install line above.

## Usage

Just run `heft` with no arguments. It scans your home directory, ranks everything by staleness, and gives you three choices: pick items interactively, auto clean all stale items, or quit.

```bash
heft                          # interactive scan and clean
heft scan                     # scan only, no cleanup
heft scan --sort staleness    # rank results by age x size
heft clean                    # interactive cleanup by category
heft clean --stale            # only target stale entries
heft clean --dry-run          # preview what would be deleted
heft audit                    # full drive audit with TUI explorer
```

Every scan saves to a local SQLite database automatically. Compare any two snapshots to see what changed.

```bash
heft diff                     # compare last two scans
heft report --list            # list all saved snapshots
heft report --id 3            # view a specific snapshot
```

## What it finds

| Category | Detected |
|---|---|
| **Project artifacts** | `node_modules`, `target`, `.venv`, `__pycache__`, `vendor`, `bin`/`obj` (.NET), gradle/maven builds, Xcode DerivedData |
| **Package caches** | npm, yarn, pnpm, pip, cargo, homebrew, go modules, maven, gradle, NuGet |
| **Docker** | images, containers, volumes, build cache, Desktop VM disk (macOS), WSL2 virtual disks (Windows) |
| **IDE data** | VSCode caches, Android AVD images, Android SDK |

## Safety

Active projects are protected by default. heft checks git recency, file modification times, and running processes before recommending anything for cleanup. Each entry gets a safety tier (disposable, rebuildable, caution, or user data) and only the safe tiers are eligible for automatic cleanup.

Every deletion path is validated: must be absolute, must be under your home directory, symlinks are never followed. Nothing happens without your confirmation unless you pass `--yes` or `--auto`.

## Config

Persistent settings go in `~/.config/heft/config.toml`. CLI flags always take priority.

```toml
[scan]
roots = ["~/code"]
verbose = true

[detectors]
docker = false
xcode = false

[staleness]
default_factor = 3.0

[activity]
window = "7d"
```

## Scripting

```bash
heft scan --json | jq '.entries[] | select(.size_bytes > 1073741824)'
heft scan --progressive       # stream results as detectors finish
heft clean --yes              # skip all prompts
heft --auto --min-size 500MB  # auto clean stale items above 500 MB
heft audit --export csv       # export full audit to csv
```

MIT license
