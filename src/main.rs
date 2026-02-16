use clap::Parser;
use heft::cli::{Cli, Command};
use heft::config::Config;
use heft::scan;
use heft::report;
use heft::clean;
use heft::snapshot;
use heft::util;
use heft::store::diff::{DiffResult, DiffType};
use heft::scan::detector::BloatCategory;
use std::collections::HashMap;

fn print_diff(result: &DiffResult) {
    // format timestamps
    let from_date = chrono::DateTime::from_timestamp(result.from_timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let to_date = chrono::DateTime::from_timestamp(result.to_timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("\nComparing snapshots:");
    println!("  From: #{} ({})", result.from_id, from_date);
    println!("  To:   #{} ({})", result.to_id, to_date);
    println!();

    if result.entries.is_empty() {
        println!("No changes detected.");
        return;
    }

    // group entries by category
    let mut by_category: HashMap<BloatCategory, Vec<&heft::store::diff::DiffEntry>> = HashMap::new();
    for entry in &result.entries {
        by_category.entry(entry.category).or_default().push(entry);
    }

    // sort categories for consistent output
    let mut categories: Vec<_> = by_category.keys().collect();
    categories.sort_by_key(|c| format!("{:?}", c));

    // print by category
    for category in categories {
        let entries = by_category.get(category).unwrap();

        println!("{}:", format!("{:?}", category));

        // separate by diff type
        let mut grew: Vec<_> = entries.iter().filter(|e| matches!(e.diff_type, DiffType::Grew)).collect();
        let mut shrank: Vec<_> = entries.iter().filter(|e| matches!(e.diff_type, DiffType::Shrank)).collect();
        let mut new: Vec<_> = entries.iter().filter(|e| matches!(e.diff_type, DiffType::New)).collect();
        let mut gone: Vec<_> = entries.iter().filter(|e| matches!(e.diff_type, DiffType::Gone)).collect();

        // sort by absolute delta (biggest changes first)
        grew.sort_by_key(|e| -(e.delta));
        shrank.sort_by_key(|e| e.delta); // already negative, so smallest (most negative) first
        new.sort_by_key(|e| -(e.delta));
        gone.sort_by_key(|e| e.delta);

        // print grew
        if !grew.is_empty() {
            for entry in grew {
                println!("  📈 {} grew {} → {} (+{})",
                    entry.name,
                    util::format_bytes(entry.old_size),
                    util::format_bytes(entry.new_size),
                    util::format_bytes(entry.delta as u64)
                );
            }
        }

        // print shrank
        if !shrank.is_empty() {
            for entry in shrank {
                println!("  📉 {} shrank {} → {} ({})",
                    entry.name,
                    util::format_bytes(entry.old_size),
                    util::format_bytes(entry.new_size),
                    util::format_bytes((-entry.delta) as u64)
                );
            }
        }

        // print new
        if !new.is_empty() {
            for entry in new {
                println!("  🆕 {} appeared ({})",
                    entry.name,
                    util::format_bytes(entry.new_size)
                );
            }
        }

        // print gone
        if !gone.is_empty() {
            for entry in gone {
                println!("  ✅ {} cleaned up (was {})",
                    entry.name,
                    util::format_bytes(entry.old_size)
                );
            }
        }

        println!();
    }

    // net change summary
    println!("Net change: {}", if result.net_change >= 0 {
        format!("+{} of new bloat", util::format_bytes(result.net_change as u64))
    } else {
        format!("{} freed", util::format_bytes((-result.net_change) as u64))
    });
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan(args) => {
            let config = Config::from_scan_args(&args);
            let result = scan::run(&config);

            // Auto-save snapshot to database
            if let Err(e) = snapshot::save_snapshot(&result) {
                if config.verbose {
                    eprintln!("warning: failed to save snapshot: {e}");
                }
            }

            report::print(&result, &config);
        }
        Command::Report(args) => {
            if args.list {
                // List all snapshots
                match snapshot::list_snapshots() {
                    Ok(snapshots) => {
                        if snapshots.is_empty() {
                            println!("No snapshots found. Run 'heft scan' to create one.");
                        } else {
                            println!("Snapshots:");
                            println!("{:<6} {:<20} {:<12} {:<12}", "ID", "Date", "Total", "Reclaimable");
                            println!("{}", "-".repeat(60));

                            for snapshot in snapshots {
                                let datetime = chrono::DateTime::from_timestamp(snapshot.timestamp, 0)
                                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                    .unwrap_or_else(|| "unknown".to_string());

                                let total = util::format_bytes(snapshot.total_bytes);
                                let reclaimable = util::format_bytes(snapshot.reclaimable_bytes);

                                println!("{:<6} {:<20} {:<12} {:<12}", snapshot.id, datetime, total, reclaimable);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error listing snapshots: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                // Show specific snapshot
                let snapshot_result = if let Some(id_str) = &args.id {
                    // Show specific snapshot by ID
                    let id: i64 = id_str.parse().expect("Invalid snapshot ID");
                    snapshot::get_snapshot(id)
                } else {
                    // Show latest snapshot (default)
                    snapshot::get_latest_snapshot()
                };

                match snapshot_result {
                    Ok(Some(snapshot)) => {
                        if args.json {
                            // Load entries for JSON output
                            let entries = snapshot::load_snapshot_entries(snapshot.id)
                                .unwrap_or_default();

                            let scan_result = scan::ScanResult {
                                entries,
                                diagnostics: vec![],
                                duration_ms: Some(snapshot.scan_duration_ms),
                                detector_timings: vec![],
                                peak_memory_bytes: snapshot.peak_memory_bytes,
                                detector_memory: vec![],
                            };

                            println!("{}", report::json::render(&scan_result));
                        } else {
                            // Human-readable output
                            let entries = snapshot::load_snapshot_entries(snapshot.id)
                                .unwrap_or_default();

                            let scan_result = scan::ScanResult {
                                entries,
                                diagnostics: vec![],
                                duration_ms: Some(snapshot.scan_duration_ms),
                                detector_timings: vec![],
                                peak_memory_bytes: snapshot.peak_memory_bytes,
                                detector_memory: vec![],
                            };

                            // Use table rendering
                            print!("{}", report::table::render(&scan_result));

                            // Show snapshot metadata
                            let datetime = chrono::DateTime::from_timestamp(snapshot.timestamp, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| "unknown".to_string());

                            println!("\nsnapshot: {} ({})", snapshot.id, datetime);
                            println!("scan duration: {:.2}s", snapshot.scan_duration_ms as f64 / 1000.0);
                            if let Some(mem) = snapshot.peak_memory_bytes {
                                println!("peak memory: {:.1} MB", mem as f64 / 1_024_f64 / 1_024_f64);
                            }
                        }
                    }
                    Ok(None) => {
                        eprintln!("No snapshots found. Run 'heft scan' to create one.");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error loading snapshot: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
        Command::Clean(args) => {
            // run a fresh scan to get current state
            let config = Config::default();
            let scan_result = scan::run(&config);

            // determine clean mode based on flags
            let mode = if args.yes {
                clean::CleanMode::Execute
            } else if args.dry_run {
                clean::CleanMode::DryRun
            } else {
                clean::CleanMode::Interactive
            };

            // run cleanup
            let clean_result = clean::run(&scan_result, mode, args.category.clone());

            // print results (skip for interactive mode - already printed)
            if !matches!(mode, clean::CleanMode::Interactive) {
                for item in &clean_result.deleted {
                    println!("{item}");
                }

                if !clean_result.errors.is_empty() {
                    eprintln!("\nerrors encountered:");
                    for error in &clean_result.errors {
                        eprintln!("  {error}");
                    }
                }

                let mb_freed = clean_result.bytes_freed as f64 / 1_024_f64 / 1_024_f64;
                if args.dry_run {
                    println!("\nwould free: {mb_freed:.2} MB");
                } else {
                    println!("\nfreed: {mb_freed:.2} MB");
                }
            } else if !clean_result.errors.is_empty() {
                eprintln!("\nerrors encountered:");
                for error in &clean_result.errors {
                    eprintln!("  {error}");
                }
            }
        }
        Command::Diff(args) => {
            use heft::store::diff;

            // determine which snapshots to compare
            let (from_id, to_id) = if let (Some(from_str), Some(to_str)) = (&args.from, &args.to) {
                // explicit snapshot IDs provided
                let from: i64 = from_str.parse().unwrap_or_else(|_| {
                    eprintln!("Invalid 'from' snapshot ID: '{}'. Must be a number.", from_str);
                    std::process::exit(1);
                });
                let to: i64 = to_str.parse().unwrap_or_else(|_| {
                    eprintln!("Invalid 'to' snapshot ID: '{}'. Must be a number.", to_str);
                    std::process::exit(1);
                });
                (from, to)
            } else {
                // default: compare two most recent snapshots
                match snapshot::list_snapshots() {
                    Ok(snapshots) => {
                        if snapshots.len() < 2 {
                            eprintln!("Need at least 2 snapshots to compare. Run 'heft scan' a few times.");
                            std::process::exit(1);
                        }
                        // snapshots are ordered by timestamp DESC, so [0] is newest
                        (snapshots[1].id, snapshots[0].id)
                    }
                    Err(e) => {
                        eprintln!("Error loading snapshots: {e}");
                        std::process::exit(1);
                    }
                }
            };

            // load both snapshots
            let from_snapshot = snapshot::get_snapshot(from_id)
                .expect("Failed to load 'from' snapshot")
                .unwrap_or_else(|| {
                    eprintln!("Snapshot {from_id} not found");
                    std::process::exit(1);
                });

            let to_snapshot = snapshot::get_snapshot(to_id)
                .expect("Failed to load 'to' snapshot")
                .unwrap_or_else(|| {
                    eprintln!("Snapshot {to_id} not found");
                    std::process::exit(1);
                });

            // load entries for both snapshots
            let from_entries = snapshot::load_snapshot_entries(from_id)
                .expect("Failed to load entries for 'from' snapshot");
            let to_entries = snapshot::load_snapshot_entries(to_id)
                .expect("Failed to load entries for 'to' snapshot");

            // compare
            let diff_result = diff::compare_entries(
                &from_entries,
                &to_entries,
                from_id,
                to_id,
                from_snapshot.timestamp,
                to_snapshot.timestamp,
            );

            // format and print
            print_diff(&diff_result);
        }
    }
}
