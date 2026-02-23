use clap::{Parser, Subcommand, ValueEnum};
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

    /// Disable JSON output (overrides config file)
    #[arg(long, conflicts_with = "json", hide_short_help = true)]
    pub no_json: bool,

    /// Skip the Docker detector (shorthand for --disable docker)
    #[arg(long, default_value_t = false)]
    pub no_docker: bool,

    /// Disable specific detectors (comma-separated: docker,xcode,projects,caches)
    #[arg(long, value_delimiter = ',')]
    pub disable: Option<Vec<String>>,

    /// Per-detector timeout in seconds
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Show detailed output including diagnostics
    #[arg(long, short = 'v', default_value_t = false)]
    pub verbose: bool,

    /// Disable verbose output (overrides config file)
    #[arg(long, conflicts_with = "verbose", hide_short_help = true)]
    pub no_verbose: bool,

    /// Show progressive output as each detector completes
    #[arg(long, default_value_t = false)]
    pub progressive: bool,

    /// Disable progressive output (overrides config file)
    #[arg(long, conflicts_with = "progressive", hide_short_help = true)]
    pub no_progressive: bool,
}

#[derive(Parser)]
pub struct ReportArgs {
    /// List all snapshots
    #[arg(long, default_value_t = false)]
    pub list: bool,

    /// Show a specific snapshot by ID
    #[arg(long)]
    pub id: Option<String>,

    /// Output as JSON
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum CleanCategory {
    #[value(name = "project-artifacts")]
    ProjectArtifacts,
    #[value(name = "container-data")]
    ContainerData,
    #[value(name = "package-cache")]
    PackageCache,
    #[value(name = "ide-data")]
    IdeData,
    #[value(name = "system-cache")]
    SystemCache,
    #[value(name = "other")]
    Other,
}

#[derive(Parser)]
pub struct CleanArgs {
    /// Skip confirmation and execute deletion (conflicts with --dry-run)
    #[arg(long, default_value_t = false, conflicts_with = "dry_run")]
    pub yes: bool,

    /// Show what would be deleted without making any changes
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Only clean specific categories
    #[arg(long, value_delimiter = ',')]
    pub category: Option<Vec<CleanCategory>>,

    /// Directories to scan (defaults to home directory)
    #[arg(long, value_delimiter = ',')]
    pub roots: Option<Vec<PathBuf>>,

    /// Skip the Docker detector (shorthand for --disable docker)
    #[arg(long, default_value_t = false)]
    pub no_docker: bool,

    /// Disable specific detectors (comma-separated: docker,xcode,projects,caches)
    #[arg(long, value_delimiter = ',')]
    pub disable: Option<Vec<String>>,

    /// Per-detector timeout in seconds
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Show detailed output including diagnostics
    #[arg(long, short = 'v', default_value_t = false)]
    pub verbose: bool,

    /// Disable verbose output (overrides config file)
    #[arg(long, conflicts_with = "verbose", hide_short_help = true)]
    pub no_verbose: bool,
}

#[derive(Parser)]
pub struct DiffArgs {
    /// Starting snapshot ID for comparison
    #[arg(long)]
    pub from: Option<String>,

    /// Ending snapshot ID for comparison
    #[arg(long)]
    pub to: Option<String>,
}
