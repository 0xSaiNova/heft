# Research

Background research on disk usage patterns and existing tools.

## Existing Tools

- **ncdu** - interactive disk usage analyzer, no semantic awareness
- **dust** - fast du alternative with visualization
- **kondo** - finds project build artifacts, no Docker or caches
- **npkill** - node_modules only
- **docker system df** - Docker only
- **dua** - fast disk usage with TUI

## Known Bloat Locations

### Project Artifacts
- node_modules (npm, yarn, pnpm)
- target/ (Rust)
- __pycache__, .venv (Python)
- vendor/ (Go, PHP)
- build/, dist/ (various)
- DerivedData (Xcode)
- .gradle (Java)

### Package Caches
- ~/.npm
- ~/.cache/pip, ~/Library/Caches/pip
- ~/.cargo/registry, ~/.cargo/git
- $(brew --cache)
- ~/.cache/yarn, ~/Library/Caches/Yarn

### Docker
- /var/lib/docker (Linux)
- ~/Library/Containers/com.docker.docker (macOS)
- Docker Desktop VM disk image

## Size Estimates

Typical developer workstation:
- Docker: 10-50 GB
- node_modules across projects: 5-20 GB
- Cargo cache + targets: 5-15 GB
- Package manager caches: 2-10 GB
