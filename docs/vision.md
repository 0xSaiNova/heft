# heft — Vision

## One Sentence

A single command that shows developers exactly where their disk space went, how much they can safely reclaim, and whether last month's cleanup actually stuck.

## The Problem We Solve

Developer workstations bleed disk space silently. Docker images pile up, node_modules multiply across abandoned projects, cargo build artifacts grow without bound, package manager caches swell in hidden dotfiles. A polyglot developer on a 512 GB SSD can easily lose 40 to 80 GB to reclaimable junk without realizing it.

The current workflow is reactive and fragmented. The developer runs out of space, panics, opens a disk visualizer that shows unlabeled blocks, then manually runs five or six different cleanup commands learned from five or six different blog posts. This happens every few weeks. Nothing tracks whether the cleanup worked or where the space went afterward.

Every existing tool addresses one slice of this. ncdu shows what's big but has no idea what's safe to delete. kondo finds build artifacts but ignores Docker and caches. npkill handles node_modules only. Docker has its own prune commands but nothing connects that to the rest of the picture. No tool gives the developer a single unified view of all developer specific disk bloat, and no tool tracks how that usage changes over time.

## What heft Does

heft gives developers a semantically aware audit of their entire development environment. It knows the difference between a 4 GB target directory and a 4 GB photo library. It understands Docker's storage model, package manager caches, build artifact patterns, and IDE junk. It reports what it finds in a categorized breakdown with reclaimable estimates, prioritized by impact.

On first run, heft is immediately useful. No setup, no historical data needed, no daemon to configure. Run `heft scan`, get a full picture in 30 seconds.

Over time, heft gets more useful. It stores snapshots and can compare them to show what grew, what shrank, and whether cleanup actually freed space permanently or just temporarily.

## Design Principles

**Audit first, cleanup second.** The default action is always to show information, never to delete anything. Every other tool in this space leads with deletion. heft leads with understanding. Cleanup is available but always explicit, always opt in, always preceded by a dry run.

**Accurate over impressive.** If heft says 55 GB is reclaimable, that number needs to be real. Overestimating destroys trust permanently. It's better to conservatively report 40 GB that's genuinely safe to reclaim than to claim 60 GB that includes space the developer can't actually free without consequences.

**Fast enough to use casually.** A full scan should complete in under 30 seconds for a typical workstation. Developers won't build a habit around a tool that takes minutes to run. Progressive output matters. Show results as each detector finishes rather than waiting for the slowest one.

**One binary, no runtime dependencies.** heft ships as a single statically linked binary. No Python, no Node, no Docker required to run heft itself. It queries Docker if Docker is present but degrades gracefully if it isn't.

**Cross platform but not cross platform from day one.** macOS and Linux first. Windows support is a separate milestone. Trying to ship all three simultaneously is how projects never ship at all.

## What heft Is Not

heft is not a generic disk usage analyzer. Tools like ncdu and dust already do that well. heft doesn't try to tell you about your movies folder or your downloads directory. It focuses exclusively on developer toolchain bloat where it can add semantic understanding that generic tools can't.

heft is not a system cleaner. It doesn't touch system caches, browser data, or operating system files. Those are a different problem with different risk profiles.

heft is not a replacement for kondo, npkill, or docker prune. Those tools are fine at what they do. heft is the diagnostic layer that sits above them, showing the full picture and helping the developer decide where to focus their cleanup effort.

## Target User

Any developer who has ever spent 30 minutes figuring out what to delete when their disk filled up. The sweet spot is polyglot developers on 256 to 512 GB SSDs who use two or more of: Docker, Node.js, Rust, Python, Homebrew. This is most professional developers on macOS and a large portion on Linux.

## Success Criteria

heft is successful if a developer can install it, run one command, and within 30 seconds have a clear understanding of how much space is reclaimable on their machine, broken down by category, with enough context to decide what to clean up and how.

The stretch goal is that developers run heft monthly as a habit rather than only when their disk is full, because the historical tracking gives them value even when they're not in crisis mode.

## Milestones

### v0.1 — Proof of value

Project detector and cache detector. TUI table output. Scans home directory for known build artifact and cache patterns. Reports categorized sizes. macOS and Linux. No cleanup, no Docker, no snapshots.

This version validates two things: that the detector trait design works, and that a categorized audit is more useful than a raw du listing.

### v0.2 — The big one

Docker detector. Queries the Docker daemon for image, container, volume, and build cache sizes. Reports reclaimable estimates using Docker's own accounting. This is the highest impact single feature because Docker is typically the largest consumer.

### v0.3 — Cleanup

Cleanup engine with dry run as default. Interactive mode that asks for confirmation per category. Supports project artifacts and caches first, Docker cleanup in a follow up.

### v0.4 — Memory

SQLite snapshot store. Diff engine comparing two snapshots. `heft diff` shows what changed since the last scan. This is the feature nobody else has and the reason to keep using heft after the first cleanup.

### v0.5 — Polish

JSON output for scripting and piping. Config file for custom scan roots and detector preferences. Platform specific detectors for Xcode on macOS and WSL2 overhead on Windows if Windows support begins.

### v1.0 — Stable

All core detectors, cleanup, snapshots, diff, cross platform support. Stable CLI interface that won't break between releases. Published to crates.io and Homebrew.
