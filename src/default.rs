//! Default flow orchestrator for bare `heft` invocation.

use std::io::IsTerminal;

use crate::big;
use crate::clean;
use crate::cli::Cli;
use crate::config::Config;
use crate::report;
use crate::scan::{self, ScanResult};
use crate::staleness::compute_staleness;
use crate::store::snapshot::Store;
use crate::summary;
use crate::util::parse_size;

pub fn run_default(cli: &Cli, config: Config) {
    let min_bytes = parse_size(&cli.min_size).unwrap_or_else(|e| {
        eprintln!("Invalid --min-size: {e}");
        std::process::exit(1);
    });

    // run detector scan
    let mut result = scan::run(&config);

    // find big files outside detector coverage
    let mut big_files = big::find_big_files(&config.roots, min_bytes);
    big::dedup_big_files(&mut big_files, &result.entries);
    let new_entries: Vec<_> = big_files.into_iter().map(big::big_file_to_entry).collect();

    // compute staleness only for the new big file entries
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let staleness_cfg = config.staleness.clone().unwrap_or_default();
    for mut entry in new_entries {
        entry.staleness_score = Some(compute_staleness(
            entry.size_bytes,
            entry.last_modified,
            now,
            &staleness_cfg,
        ));
        result.entries.push(entry);
    }

    // sort all entries by staleness descending
    result.entries.sort_by(|a, b| {
        let sa = a.staleness_score.unwrap_or(0.0);
        let sb = b.staleness_score.unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    // save snapshot
    if let Ok(mut store) = Store::open() {
        let _ = store.save_snapshot(&result);
    }

    // json mode: print and exit
    if cli.json {
        println!("{}", report::json::render(&result));
        return;
    }

    summary::print_summary(&result.entries);

    if cli.dry_run {
        return;
    }

    if !std::io::stdout().is_terminal() {
        return;
    }

    if cli.auto {
        let stale: Vec<_> = result
            .entries
            .iter()
            .filter(|e| {
                if !cli.include_active && e.active == Some(true) {
                    return false;
                }
                e.staleness_score.unwrap_or(0.0) > 0.0
            })
            .cloned()
            .collect();
        if stale.is_empty() {
            println!("  Nothing stale to clean.");
            return;
        }
        confirm_and_clean(stale, cli.include_active);
        return;
    }

    // interactive prompt
    summary::print_prompt();
    let key = read_key();
    match key.as_str() {
        "a" => {
            let stale: Vec<_> = result
                .entries
                .iter()
                .filter(|e| {
                    if !cli.include_active && e.active == Some(true) {
                        return false;
                    }
                    e.staleness_score.unwrap_or(0.0) > 0.0
                })
                .cloned()
                .collect();
            if stale.is_empty() {
                println!("  Nothing stale to clean.");
            } else {
                confirm_and_clean(stale, cli.include_active);
            }
        }
        "i" => {
            let picked = crate::picker::run_picker(&result.entries, cli.include_active);
            if !picked.is_empty() {
                confirm_and_clean(picked, cli.include_active);
            }
        }
        _ => {}
    }
}

fn confirm_and_clean(entries: Vec<crate::scan::detector::BloatEntry>, include_active: bool) {
    let mut selected = ScanResult::empty();
    selected.entries = entries;
    let opts = clean::CleanOptions {
        category_filter: None,
        include_active,
    };
    let clean_result = clean::run(&selected, clean::CleanMode::Execute, opts);
    for item in &clean_result.deleted {
        println!("  {item}");
    }
    for err in &clean_result.errors {
        eprintln!("  error: {err}");
    }
    if clean_result.bytes_freed > 0 {
        println!(
            "\n  Freed {}",
            crate::util::format_bytes(clean_result.bytes_freed)
        );
    }
}

/// Read a single key from the terminal. Uses crossterm when available,
/// falls back to reading a line from stdin.
#[cfg(feature = "tui")]
fn read_key() -> String {
    use crossterm::event::{self, Event, KeyCode};
    if crossterm::terminal::enable_raw_mode().is_ok() {
        let result = match event::read() {
            Ok(Event::Key(key_event)) => match key_event.code {
                KeyCode::Char(c) => c.to_string(),
                KeyCode::Enter => "\n".to_string(),
                KeyCode::Esc => "q".to_string(),
                _ => String::new(),
            },
            _ => String::new(),
        };
        let _ = crossterm::terminal::disable_raw_mode();
        println!();
        result
    } else {
        read_line_fallback()
    }
}

#[cfg(not(feature = "tui"))]
fn read_key() -> String {
    read_line_fallback()
}

fn read_line_fallback() -> String {
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        input.trim().to_string()
    } else {
        String::new()
    }
}
