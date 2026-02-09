use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "heft")]
#[command(about = "A disk space auditor for developers")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run all detectors and output results
    Scan(ScanArgs),

    /// Display the most recent scan or a specific snapshot
    Report(ReportArgs),

    /// Remove reclaimable items
    Clean(CleanArgs),

    /// Compare two snapshots
    Diff(DiffArgs),
}

#[derive(Parser)]
pub struct ScanArgs {
    /// Directories to scan (defaults to home directory)
    #[arg(long, value_delimiter = ',')]
    pub roots: Option<Vec<PathBuf>>,

    /// Output as JSON instead of table
    #[arg(long, default_value_t = false)]
    pub json: bool,

    /// Skip the Docker detector
    #[arg(long, default_value_t = false)]
    pub no_docker: bool,

    /// Per-detector timeout in seconds
    #[arg(long, default_value_t = 30)]
    pub timeout: u64,

    /// Show detailed output including diagnostics
    #[arg(long, short = 'v', default_value_t = false)]
    pub verbose: bool,
}

#[derive(Parser)]
pub struct ReportArgs {
    /// Show most recent snapshot (default behavior)
    #[arg(long, default_value_t = true)]
    pub latest: bool,

    /// Show a specific snapshot by ID
    #[arg(long)]
    pub id: Option<String>,

    /// Output as JSON
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Parser)]
pub struct CleanArgs {
    /// Skip confirmation and execute deletion
    #[arg(long, default_value_t = false)]
    pub yes: bool,

    /// Only clean specific categories
    #[arg(long, value_delimiter = ',')]
    pub category: Option<Vec<String>>,
}

impl CleanArgs {
    /// returns true if this is a dry run (show what would be deleted)
    pub fn is_dry_run(&self) -> bool {
        !self.yes
    }
}

#[derive(Parser)]
pub struct DiffArgs {
    /// Compare the two most recent snapshots (default behavior)
    #[arg(long, default_value_t = true)]
    pub last: bool,

    /// Starting snapshot ID for comparison
    #[arg(long)]
    pub from: Option<String>,

    /// Ending snapshot ID for comparison
    #[arg(long)]
    pub to: Option<String>,
}
