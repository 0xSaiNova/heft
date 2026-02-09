# heft

finds build artifacts and caches eating your disk space so you know what to delete.

## what it does

scans your filesystem for:
- project artifacts: node_modules, cargo target/, python venvs, gradle builds, xcode deriveddata
- package caches: npm, pip, cargo, homebrew, maven, gradle
- shows size and last modified time for each

outputs as table or json.

## usage

```bash
heft scan                              # scan home directory
heft scan --roots ~/code ~/projects    # scan specific directories
heft scan --json                       # output as json
heft clean                             # dry-run shows what would be deleted
heft clean --yes                       # actually delete
heft clean --category package-cache    # only clean caches
```

## current status

- project artifact detection: working
- cache detection: working
- json/table output: working
- cleanup engine: working
- docker detection: placeholder
- snapshot storage: not started

## whats next

- fix remaining bugs (overflow, permission errors, platform handling)
- implement docker detector
- add snapshot/diff functionality
- progressive output during scan
- windows support

## building

```bash
cargo build --release
./target/release/heft scan
```

## license

MIT
