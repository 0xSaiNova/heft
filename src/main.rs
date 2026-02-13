use clap::Parser;
use heft::cli::{Cli, Command};
use heft::config::Config;
use heft::scan;
use heft::report;
use heft::clean;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan(args) => {
            let config = Config::from_scan_args(&args);
            let result = scan::run(&config);
            report::print(&result, &config);
        }
        Command::Report(args) => {
            println!("report: snapshot_id={:?}, json={}", args.id, args.json);
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
