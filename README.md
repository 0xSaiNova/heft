# heft

**Your disk is full. Again.**

You've got 247 node_modules folders, docker is eating 50GB, and you have no idea which projects are even still active. Sound familiar?

heft finds what's actually eating your disk space and lets you clean it up intelligently. No more guessing what's safe to delete.

## why heft

**Fast and accurate:** Scans your entire home directory in seconds, not minutes. No recursion limit games.

**Actually useful output:** See exactly where your space went - build artifacts, package caches, docker bloat, IDE data. Not just "some files somewhere."

**Interactive cleanup:** Review by category before deleting. No all-or-nothing choices. No regrets.

**Built for developers:** Understands your tools - npm, cargo, docker, gradle, pip, homebrew, maven, go modules, and more.

## what it finds

**Project artifacts** (the stuff that rebuilds anyway):
- `node_modules/` and `target/` directories that add up fast
- Python virtual environments sitting in old projects
- Gradle and Maven build caches
- Xcode DerivedData taking gigabytes

**Package manager caches** (safe to clear, they redownload):
- npm, yarn, pnpm stores
- cargo registry and git checkouts
- pip, homebrew, go module caches
- gradle and maven local repositories

**Docker bloat** (the silent disk killer):
- Unused images, stopped containers
- Dangling volumes, build cache
- Docker Desktop VM files (macOS/Windows)

**IDE and tooling data:**
- VSCode extensions and cache
- Language server caches

## quick start

```bash
# see what's eating your disk
heft scan

# clean it up interactively
heft clean

# or get specific
heft clean --category package-cache    # just clear caches
heft clean --category container-data   # docker cleanup
heft clean --dry-run                   # preview without prompts
```

## interactive cleanup mode

The default cleanup experience lets you approve each category before deletion:

```
Found 21.2 GB reclaimable across 3 categories:

ProjectArtifacts: 8.1 GB (18 items)
  Delete? [y/n]: y

PackageCache: 5.4 GB (6 items)
  Delete? [y/n]: n

ContainerData: 7.7 GB (3 items)
  Delete? [y/n]: y

Freed 15.8 GB
```

Safer than `--yes`, faster than manual filtering. You stay in control.

## current status (v0.3.0)

**Working and tested:**
- Multi-platform detection (Linux, macOS, Windows)
- Project artifact scanning (11+ project types)
- Package cache detection (10+ package managers)
- Docker object detection and cleanup
- Interactive cleanup with category filtering
- JSON output for automation
- Security hardening (path validation, symlink protection)

**Coming soon:**
- Snapshot storage (track what's growing over time)
- Diff engine (see what changed between scans)
- Per-detector timing and progress output

**Platform support:**
- Linux: full support, thoroughly tested
- macOS: full support, community testing ongoing
- Windows: basic support, paths verified

## safety

heft takes cleanup seriously:
- Only touches known build artifacts and caches
- Validates paths before deletion (absolute paths only, under home or temp)
- Refuses to follow symlinks (prevents accidental system deletion)
- Never deletes source code or project files
- Interactive mode lets you review before any deletion
- All cleanup operations use platform-standard commands

## installation

**From source (recommended for now):**

```bash
git clone https://github.com/0xSaiNova/heft.git
cd heft
cargo build --release
./target/release/heft scan
```

**Add to PATH (optional):**

```bash
# add to your shell profile
export PATH="$PATH:/path/to/heft/target/release"
```

*Publishing to crates.io coming with v1.0 after more platform testing.*

## examples

**Basic scan:**
```bash
$ heft scan
Found 15.3 GB reclaimable across 24 items

ProjectArtifacts:
  node_modules (old-project)           2.1 GB   120 days ago
  target (rust-experiments)            890 MB   45 days ago

PackageCache:
  npm cache                            3.2 GB
  cargo registry                       1.8 GB

ContainerData:
  Docker Images                        7.3 GB
```

**Target specific directories:**
```bash
heft scan --roots ~/code ~/projects
```

**JSON for scripting:**
```bash
heft scan --json | jq '.entries[] | select(.size_bytes > 1000000000)'
```

**Cleanup specific categories:**
```bash
heft clean --category project-artifacts    # only old build outputs
heft clean --category container-data       # docker cleanup
heft clean --yes                           # skip prompts, delete all
```

## categories

- `project-artifacts` - Build outputs, dependencies (node_modules, target/)
- `package-cache` - Package manager caches (npm, cargo, pip, etc)
- `container-data` - Docker images, containers, volumes, build cache
- `ide-data` - VSCode and other IDE caches
- `system-cache` - System-level caches

## performance

Tested on real developer machines:
- **Scan time:** ~3 seconds for typical home directory
- **Memory usage:** ~27 MB peak
- **Binary size:** 1.6 MB
- **No dependencies:** Self-contained binary, no runtime required

## contributing

Found a bug? Have a feature request? Want to add support for a new package manager?

Open an issue at https://github.com/0xSaiNova/heft/issues

**Good first issues:**
- Add detection for your favorite package manager
- Improve platform-specific path handling
- Add new project type detection

Check [docs/fix-roadmap.md](docs/fix-roadmap.md) to see what's being worked on.

## why "heft"

Because that's what your bloated disk has. Heft finds it, measures it, and helps you get rid of it.

## license

MIT - see [LICENSE](LICENSE) for details
