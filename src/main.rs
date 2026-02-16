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

fn category_label(cat: &BloatCategory) -> &'static str {
    match cat {
        BloatCategory::ProjectArtifacts => "Project Artifacts",
        BloatCategory::ContainerData => "Container Data",
        BloatCategory::PackageCache => "Package Cache",
        BloatCategory::IdeData => "IDE Data",
        BloatCategory::SystemCache => "System Cache",
        BloatCategory::Other => "Other",
    }
}

fn print_diff(result: &DiffResult) {
    let from_date = chrono::DateTime::from_timestamp(result.from_timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let to_date = chrono::DateTime::from_timestamp(result.to_timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("\nComparing snapshots:");
    println!("  From: #{} ({from_date})", result.from_id);
    println!("  To:   #{} ({to_date})", result.to_id);
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
    categories.sort_by_key(|c| category_label(c));

    for category in categories {
        let Some(entries) = by_category.get(category) else { continue };

        println!("{}:", category_label(category));

        let mut grew: Vec<_> = entries.iter().filter(|e| matches!(e.diff_type, DiffType::Grew)).collect();
        let mut shrank: Vec<_> = entries.iter().filter(|e| matches!(e.diff_type, DiffType::Shrank)).collect();
        let mut new: Vec<_> = entries.iter().filter(|e| matches!(e.diff_type, DiffType::New)).collect();
        let mut gone: Vec<_> = entries.iter().filter(|e| matches!(e.diff_type, DiffType::Gone)).collect();

        grew.sort_by_key(|e| -(e.delta));
        shrank.sort_by_key(|e| e.delta);
        new.sort_by_key(|e| -(e.delta));
        gone.sort_by_key(|e| e.delta);

        for entry in grew {
            println!("  [+] {} grew {} -> {} (+{})",
                entry.name,
                util::format_bytes(entry.old_size),
                util::format_bytes(entry.new_size),
                util::format_bytes(entry.delta.unsigned_abs())
            );
        }

        for entry in shrank {
            println!("  [-] {} shrank {} -> {} (-{})",
                entry.name,
                util::format_bytes(entry.old_size),
                util::format_bytes(entry.new_size),
                util::format_bytes(entry.delta.unsigned_abs())
            );
        }

        for entry in new {
            println!("  [new] {} appeared ({})",
                entry.name,
                util::format_bytes(entry.new_size)
            );
        }

        for entry in gone {
            println!("  [gone] {} cleaned up (was {})",
                entry.name,
                util::format_bytes(entry.old_size)
            );
        }

        println!();
    }

    // net change summary
    if result.net_change >= 0 {
        println!("Net change: +{} of new bloat", util::format_bytes(result.net_change.unsigned_abs()));
    } else {
        println!("Net change: {} freed", util::format_bytes(result.net_change.unsigned_abs()));
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan(args) => {
            let config = Config::from_scan_args(&args);
            let result = scan::run(&config);

            if let Err(e) = snapshot::save_snapshot(&result) {
                if config.verbose {
                    eprintln!("warning: failed to save snapshot: {e}");
                }
            }

            report::print(&result, &config);
        }
        Command::Report(args) => {
            if args.list {
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
                let snapshot_result = if let Some(id_str) = &args.id {
                    let id: i64 = id_str.parse().unwrap_or_else(|_| {
                        eprintln!("Invalid snapshot ID: '{id_str}'. Must be a number.");
                        std::process::exit(1);
                    });
                    snapshot::get_snapshot(id)
                } else {
                    snapshot::get_latest_snapshot()
                };

                match snapshot_result {
                    Ok(Some(snapshot)) => {
                        let entries = snapshot::load_snapshot_entries(snapshot.id)
                            .unwrap_or_default();

                        let scan_result = scan::ScanResult {
                            entries,
                            diagnostics: vec![],
                            duration_ms: Some(snapshot.scan_duration_ms as u128),
                            detector_timings: vec![],
                            peak_memory_bytes: snapshot.peak_memory_bytes,
                            detector_memory: vec![],
                        };

                        if args.json {
                            println!("{}", report::json::render(&scan_result));
                        } else {
                            print!("{}", report::table::render(&scan_result));

                            let datetime = chrono::DateTime::from_timestamp(snapshot.timestamp, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| "unknown".to_string());

                            println!("\nsnapshot: {} ({datetime})", snapshot.id);
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
            let config = Config::default();
            let scan_result = scan::run(&config);

            let mode = if args.yes {
                clean::CleanMode::Execute
            } else if args.dry_run {
                clean::CleanMode::DryRun
            } else {
                clean::CleanMode::Interactive
            };

            let clean_result = clean::run(&scan_result, mode, args.category.clone());

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

            // validate that --from and --to are used together
            if args.from.is_some() != args.to.is_some() {
                eprintln!("Both --from and --to must be specified together.");
                std::process::exit(1);
            }

            let (from_id, to_id) = if let (Some(from_str), Some(to_str)) = (&args.from, &args.to) {
                let from: i64 = from_str.parse().unwrap_or_else(|_| {
                    eprintln!("Invalid 'from' snapshot ID: '{from_str}'. Must be a number.");
                    std::process::exit(1);
                });
                let to: i64 = to_str.parse().unwrap_or_else(|_| {
                    eprintln!("Invalid 'to' snapshot ID: '{to_str}'. Must be a number.");
                    std::process::exit(1);
                });
                (from, to)
            } else {
                match snapshot::list_snapshots() {
                    Ok(snapshots) => {
                        if snapshots.len() < 2 {
                            eprintln!("Need at least 2 snapshots to compare. Run 'heft scan' a few times.");
                            std::process::exit(1);
                        }
                        (snapshots[1].id, snapshots[0].id)
                    }
                    Err(e) => {
                        eprintln!("Error loading snapshots: {e}");
                        std::process::exit(1);
                    }
                }
            };

            let from_snapshot = match snapshot::get_snapshot(from_id) {
                Ok(Some(s)) => s,
                Ok(None) => {
                    eprintln!("Snapshot {from_id} not found");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error loading snapshot {from_id}: {e}");
                    std::process::exit(1);
                }
            };

            let to_snapshot = match snapshot::get_snapshot(to_id) {
                Ok(Some(s)) => s,
                Ok(None) => {
                    eprintln!("Snapshot {to_id} not found");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error loading snapshot {to_id}: {e}");
                    std::process::exit(1);
                }
            };

            let from_entries = match snapshot::load_snapshot_entries(from_id) {
                Ok(entries) => entries,
                Err(e) => {
                    eprintln!("Error loading entries for snapshot {from_id}: {e}");
                    std::process::exit(1);
                }
            };

            let to_entries = match snapshot::load_snapshot_entries(to_id) {
                Ok(entries) => entries,
                Err(e) => {
                    eprintln!("Error loading entries for snapshot {to_id}: {e}");
                    std::process::exit(1);
                }
            };

            let diff_result = diff::compare_entries(
                &from_entries,
                &to_entries,
                from_id,
                to_id,
                from_snapshot.timestamp,
                to_snapshot.timestamp,
            );

            print_diff(&diff_result);
        }
    }
}
