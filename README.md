# heft

**Your disk is full. Again.**

heft finds what's eating your disk and lets you clean it up. Build artifacts, package caches, docker bloat — it finds all of it in seconds.

## install

```bash
git clone https://github.com/0xSaiNova/heft.git
cd heft && cargo build --release
# optionally add to PATH
cp target/release/heft ~/.local/bin/
```

## usage

```bash
heft scan                              # find the bloat
heft clean                             # clean it up interactively
heft clean --category container-data   # just docker
heft clean --dry-run                   # preview first
heft scan --json                       # pipe to jq, scripts, etc
```

## snapshots

every scan saves automatically. come back later and see what changed.

```bash
heft report --list          # all saved snapshots
heft report --id 3          # replay a specific scan
heft diff                   # compare two most recent
heft diff --from 1 --to 5   # compare specific snapshots
```

```
Comparing snapshots:
  From: #1 (2026-02-10 14:30:00)
  To:   #5 (2026-02-15 09:15:00)

Package Cache:
  [+] npm cache grew 1.2 GB -> 1.8 GB (+600 MB)
  [-] cargo registry shrank 2.1 GB -> 1.4 GB (-700 MB)

Project Artifacts:
  [new] node_modules (new-project) appeared (450 MB)
  [gone] target (old-experiment) cleaned up (was 890 MB)

Net change: 540 MB freed
```

## what it detects

| Category | Examples |
|----------|----------|
| Project artifacts | `node_modules/`, `target/`, venvs, gradle/maven builds, DerivedData |
| Package caches | npm, yarn, pnpm, cargo, pip, homebrew, go, maven, gradle |
| Docker | images, containers, volumes, build cache, Desktop VM files |
| IDE data | VSCode extensions and caches, language servers |

## performance

- ~3 second scan on a typical home directory
- ~27 MB peak memory
- self-contained binary, no runtime dependencies

## safety

- never touches source code or project files
- validates paths before deletion (absolute, under home/temp only)
- refuses to follow symlinks
- interactive mode by default — you approve each category

## license

MIT
