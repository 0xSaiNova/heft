use clap::Parser;
use heft::cli::{Cli, Command};
use heft::config::Config;
use heft::scan;
use heft::report;
use heft::clean;
use heft::snapshot;
use heft::util;

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
            println!("diff: from={:?}, to={:?}", args.from, args.to);
        }
    }
}
