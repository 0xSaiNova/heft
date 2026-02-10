# Fix Roadmap

tracking the order we're tackling issues and why. this keeps us focused and makes sure we're building the right things in the right order.

## Status Summary (Updated: Feb 10, 2026)

**Phase 1: COMPLETE ✅**
- all detection accuracy bugs fixed
- tests passing
- critical security vulnerabilities patched

**Phase 2: IN PROGRESS - Docker Support (v0.2 milestone)**
- [#3](https://github.com/0xSaiNova/heft/issues/3) - implement docker detector ✅
- [#47](https://github.com/0xSaiNova/heft/issues/47) - fix docker object types ✅
- [#42](https://github.com/0xSaiNova/heft/issues/42) - docker desktop vm detection (remaining)

**Phase 3: Cleanup Safety** - 8 open issues ready to tackle after docker

---

## phase 1: get the basics solid ✅ COMPLETE

### [#45](https://github.com/0xSaiNova/heft/issues/45) - fix broken tests ✅ FIXED
tests won't compile after adding the verbose field. literally blocks everything else because we can't validate any changes. takes like 5 minutes to fix - just add `verbose: false` to all the test Config structs.

**status: fixed - added verbose: false to all 11 Config initializers in tests. all tests pass now**

### [#55](https://github.com/0xSaiNova/heft/issues/55), [#56](https://github.com/0xSaiNova/heft/issues/56) - critical security vulnerabilities in cleanup ✅ FIXED
found two serious security bugs in the cleanup code that could wipe your entire system:
- symlink vulnerability: delete_filesystem_path() follows symlinks, so if someone replaces a bloat dir with a symlink to / we'll delete everything
- TOCTOU race: paths can change between scan and clean. user scans, approves cleanup, but between those operations an attacker could replace the directory with a symlink

**status: fixed - switched to symlink_metadata() which doesn't follow symlinks. now refuse to delete any symlinks. if path is replaced with symlink between scan and clean, we reject deletion with error. mitigates the critical attack vector. full TOCTOU fix would need device+inode verification but symlink check handles the dangerous cases.**

location: src/clean/mod.rs:93-119

### [#49](https://github.com/0xSaiNova/heft/issues/49), [#50](https://github.com/0xSaiNova/heft/issues/50), [#51](https://github.com/0xSaiNova/heft/issues/51), [#52](https://github.com/0xSaiNova/heft/issues/52) - fix detection accuracy bugs ✅ FIXED
bunch of issues where we're either flagging wrong stuff or silently missing errors:
- build directories getting false positives
- DerivedData matching too broadly
- metadata errors ignored
- walkdir errors swallowed

need to fix these before users start trusting the tool. better to get it right now than deal with "why did you delete my folder" bug reports later. all in the same area of code so knock them out together.

**status: fixed in [PR #63](https://github.com/0xSaiNova/heft/pull/63) - added verification helpers is_gradle_build_dir() and is_xcode_derived_data() to prevent false positives. metadata and walkdir errors now logged with diagnostic messages in verbose mode. changes in src/scan/projects.rs and src/scan/caches.rs**

## phase 2: ship docker support (v0.2)

### [#3](https://github.com/0xSaiNova/heft/issues/3) - implement docker detector ✅ FIXED
this is the big one. vision doc calls it v0.2 milestone. docker is where most of the space goes (10-50gb typically according to research.md). users need this.

**status: fixed in [PR #65](https://github.com/0xSaiNova/heft/pull/65) - implemented docker detector using `docker system df --format json`. detects images, containers, volumes, and build cache with total and reclaimable sizes. handles docker not installed, daemon not running, and permission errors gracefully. changes in src/scan/docker.rs.**

### [#47](https://github.com/0xSaiNova/heft/issues/47) - fix docker object types ✅ FIXED
docker cleanup is hardcoded for images only. need to handle containers, volumes, build cache too. better to get the architecture right now than refactor later when cleanup is already being used.

**status: fixed - cleanup now handles all docker aggregate types (images, containers, volumes, build cache) using docker prune commands. changes in src/clean/mod.rs to allow docker aggregates through filter and added delete_docker_aggregate() function.**

### [#42](https://github.com/0xSaiNova/heft/issues/42) - docker desktop vm detection
macos and windows users have these huge vm disk files (30-60gb) that the docker api doesn't report. complements the docker detector by catching the physical disk usage.

**milestone: with these three done, we hit v0.2**

## phase 3: make cleanup safe

once the critical security issues ([#55](https://github.com/0xSaiNova/heft/issues/55), [#56](https://github.com/0xSaiNova/heft/issues/56)) are fixed, clean up the remaining safety issues:

### [#46](https://github.com/0xSaiNova/heft/issues/46) - replace unreachable! with error
cleanup code has a panic that could crash if something unexpected happens. should just return an error instead. quick fix, prevents potential crashes.

### [#59](https://github.com/0xSaiNova/heft/issues/59) - path validation and windows paths
add validation before deletion (path exists, is under scan root, hasn't changed). also fix windows yarn cache path that uses forward slashes. both are safety/correctness issues.

### [#57](https://github.com/0xSaiNova/heft/issues/57), [#58](https://github.com/0xSaiNova/heft/issues/58) - parser and timeout bugs
TOML parser breaks on certain Cargo.toml files, integer overflow underestimates sizes. timeout field exists but is never used anywhere. these affect accuracy and reliability.

### [#53](https://github.com/0xSaiNova/heft/issues/53) - fix zombie process leak
homebrew timeout leaves zombie processes. small resource leak but adds up. easy fix while we're already in caches.rs.

### [#54](https://github.com/0xSaiNova/heft/issues/54) - json error escaping
edge case where error messages with quotes break json output. one line fix, makes json output bulletproof.

### [#60](https://github.com/0xSaiNova/heft/issues/60) - other quality fixes
duration overflow (after 584 million years...) and manifest size checks. minor stuff but worth cleaning up. note: json escaping overlaps with [#54](https://github.com/0xSaiNova/heft/issues/54).

## phase 4: improve user experience

### [#39](https://github.com/0xSaiNova/heft/issues/39) - per-detector timing
show how long each detector takes. helps users understand why scans are slow and helps us optimize the right things.

### [#25](https://github.com/0xSaiNova/heft/issues/25) - progressive output
show results as each detector finishes instead of waiting for everything. big ux improvement for slow scans. uses the timing data from [#39](https://github.com/0xSaiNova/heft/issues/39).

## phase 5: add observability

### [#44](https://github.com/0xSaiNova/heft/issues/44) - memory monitoring
track memory usage during scans. keeps us honest about staying lightweight. stress-test.md says we should be tracking this.

### [#43](https://github.com/0xSaiNova/heft/issues/43) - benchmarking suite
automated performance tests to catch regressions. needs [#39](https://github.com/0xSaiNova/heft/issues/39) and [#44](https://github.com/0xSaiNova/heft/issues/44) to be useful since those are the metrics we want to track.

## phase 6: snapshot features (v0.4)

### [#6](https://github.com/0xSaiNova/heft/issues/6) - sqlite snapshot storage
save scan results to a database so we can compare over time. this is the killer feature that makes heft different from other tools. vision doc v0.4 milestone.

### [#7](https://github.com/0xSaiNova/heft/issues/7) - diff engine
compare two snapshots to see what grew or shrank. depends on [#6](https://github.com/0xSaiNova/heft/issues/6) obviously. completes the "track changes over time" story.

**milestone: with these we hit v0.4**

## phase 7: complete cleanup support

### [#29](https://github.com/0xSaiNova/heft/issues/29) - docker cleanup command
actually delete docker stuff from the clean command. depends on [#3](https://github.com/0xSaiNova/heft/issues/3) and [#47](https://github.com/0xSaiNova/heft/issues/47) being done first.

### [#48](https://github.com/0xSaiNova/heft/issues/48) - implement Default trait
clippy complains about our default() method. should implement the trait properly. just good housekeeping, not urgent.

### [#4](https://github.com/0xSaiNova/heft/issues/4) - verify tui table status ✅ FIXED
table output is working. closed Feb 9.

## phase 8: platform expansion

### [#22](https://github.com/0xSaiNova/heft/issues/22) - xcode detector (macos)
### [#28](https://github.com/0xSaiNova/heft/issues/28) - android studio detector
### [#23](https://github.com/0xSaiNova/heft/issues/23) - windows support

platform specific stuff. independent of each other. do after core features are solid.

## future work

### [#21](https://github.com/0xSaiNova/heft/issues/21) - config file support (v0.5)
let users customize scan roots and detector settings via config file.

### [#24](https://github.com/0xSaiNova/heft/issues/24) - publish to crates.io (v1.0)
ship it when everything's stable.

---

## the strategy

**fix critical stuff first** - tests are broken, that blocks everything

**correctness before features** - fix the bugs in what we have before adding more stuff

**high impact features next** - docker is where the space is, users need it

**safety before convenience** - fix the crashes and resource leaks before polishing ux

**build foundations properly** - get docker architecture right, get storage right, then build on top

**add observability** - measure before optimizing

**complete the vision** - snapshots/diff is what makes heft unique, prioritize getting there

this order gets us to a solid v0.2 (docker working) fast, then builds toward v0.4 (snapshots) which is the differentiator. each phase builds on the previous one and delivers value.
