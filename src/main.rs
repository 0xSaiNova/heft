use clap::Parser;
use heft::cli::{Cli, Command};
use heft::config::Config;
use heft::scan;
use heft::report;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan(args) => {
            let config = Config::from_scan_args(&args);
            let result = scan::run(&config);
            report::print(&result);
        }
        Command::Report(args) => {
            println!("report: snapshot_id={:?}, json={}", args.id, args.json);
        }
        Command::Clean(args) => {
            println!(
                "clean: dry_run={}, categories={:?}",
                args.dry_run,
                args.category
            );
        }
        Command::Diff(args) => {
            println!("diff: from={:?}, to={:?}", args.from, args.to);
        }
    }
}
