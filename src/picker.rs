//! Inline interactive checkbox picker for selecting bloat entries to clean.
//!
//! Uses crossterm raw mode to render an inline scrollable list directly
//! in the terminal. Not a full alternate screen TUI.

#[cfg(feature = "tui")]
pub fn run_picker(
    entries: &[crate::scan::detector::BloatEntry],
    include_active: bool,
) -> Vec<crate::scan::detector::BloatEntry> {
    use crossterm::event::{self, Event, KeyCode};
    use crossterm::style::{Attribute, Print, SetAttribute};
    use crossterm::terminal;
    use crossterm::{cursor, execute};
    use std::io::{stdout, Write};

    use crate::scan::detector::Location;
    use crate::staleness::staleness_label;
    use crate::util::format_bytes;

    if entries.is_empty() {
        return Vec::new();
    }

    let count = entries.len();
    let mut selected = vec![false; count];
    let mut cursor_pos: usize = 0;
    let mut scroll_offset: usize = 0;

    // Pre-select stale items (staleness_score > 0 and respecting active filter)
    for (i, e) in entries.iter().enumerate() {
        if e.staleness_score.unwrap_or(0.0) <= 0.0 {
            continue;
        }
        if e.active == Some(true) && !include_active {
            continue;
        }
        selected[i] = true;
    }

    let (_, term_rows) = terminal::size().unwrap_or((80, 24));
    // Reserve 2 lines for header and footer
    let visible = (term_rows as usize).saturating_sub(2).max(1);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let home = crate::platform::home_dir();

    let shorten_path = |loc: &Location| -> String {
        match loc {
            Location::FilesystemPath(p) => {
                if let Some(ref h) = home {
                    if let Ok(rest) = p.strip_prefix(h) {
                        return format!("~/{}", rest.display());
                    }
                }
                p.display().to_string()
            }
            Location::DockerObject(s) => s.clone(),
            Location::Aggregate(s) => s.clone(),
        }
    };

    let age_days_for = |e: &crate::scan::detector::BloatEntry| -> Option<f64> {
        e.last_modified.map(|ts| ((now - ts) as f64) / 86400.0)
    };

    let draw = |sel: &[bool],
                cur: usize,
                offset: usize,
                out: &mut std::io::Stdout|
     -> std::io::Result<()> {
        // Move cursor to start of our drawing area
        execute!(out, cursor::MoveToColumn(0))?;

        // Header
        execute!(
            out,
            Print("  Select items to clean (space: toggle, a: all stale, n: none, enter: go)\n\r"),
        )?;

        let end = (offset + visible).min(count);
        for i in offset..end {
            let e = &entries[i];
            let check = if sel[i] { "x" } else { " " };
            let size_str = format_bytes(e.size_bytes);
            let path = shorten_path(&e.location);
            let label = staleness_label(e.active, age_days_for(e));
            let prefix = if i == cur { ">" } else { " " };

            if i == cur {
                execute!(out, SetAttribute(Attribute::Bold))?;
            }
            let line = format!(
                "{prefix} [{check}]  {size_str:>10}  {name:<30}  {path}  {label}",
                name = e.name
            );
            execute!(out, Print(&line), SetAttribute(Attribute::Reset))?;

            // Clear rest of line and move to next
            execute!(
                out,
                crossterm::terminal::Clear(crossterm::terminal::ClearType::UntilNewLine),
                Print("\n\r"),
            )?;
        }

        // Clear any leftover lines if the list shrank
        for _ in end..(offset + visible) {
            execute!(
                out,
                crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                Print("\n\r"),
            )?;
        }

        // Footer with running total
        let sel_count = sel.iter().filter(|&&s| s).count();
        let sel_bytes: u64 = sel
            .iter()
            .zip(entries.iter())
            .filter(|(&s, _)| s)
            .map(|(_, e)| e.reclaimable_bytes)
            .sum();
        let footer = format!(
            "  Selected: {} ({} items)  |  enter: confirm  q: cancel",
            format_bytes(sel_bytes),
            sel_count
        );
        execute!(
            out,
            Print(&footer),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::UntilNewLine),
        )?;

        // Move cursor back up to top of our drawing area for next redraw
        let lines_drawn = (end - offset) + 1 + 1; // items + header + footer
        execute!(out, cursor::MoveUp(lines_drawn as u16))?;

        out.flush()?;
        Ok(())
    };

    if terminal::enable_raw_mode().is_err() {
        eprintln!("Failed to enable raw mode. Use --auto instead.");
        return Vec::new();
    }

    let mut out = stdout();
    // Print blank lines to reserve space, then move back up
    let total_lines = visible + 2; // header + items + footer
    for _ in 0..total_lines {
        let _ = execute!(out, Print("\n\r"));
    }
    let _ = execute!(out, cursor::MoveUp(total_lines as u16));

    let _ = draw(&selected, cursor_pos, scroll_offset, &mut out);

    let result = loop {
        let ev = match event::read() {
            Ok(ev) => ev,
            Err(_) => break Vec::new(),
        };
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break Vec::new(),
                KeyCode::Enter => {
                    let picks: Vec<_> = selected
                        .iter()
                        .zip(entries.iter())
                        .filter(|(&s, _)| s)
                        .map(|(_, e)| e.clone())
                        .collect();
                    break picks;
                }
                KeyCode::Char(' ') => {
                    // dont allow toggling active items unless include_active
                    let is_protected = entries[cursor_pos].active == Some(true) && !include_active;
                    if !is_protected {
                        selected[cursor_pos] = !selected[cursor_pos];
                        let _ = draw(&selected, cursor_pos, scroll_offset, &mut out);
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if cursor_pos + 1 < count {
                        cursor_pos += 1;
                        if cursor_pos >= scroll_offset + visible {
                            scroll_offset = cursor_pos - visible + 1;
                        }
                        let _ = draw(&selected, cursor_pos, scroll_offset, &mut out);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if cursor_pos > 0 {
                        cursor_pos -= 1;
                        if cursor_pos < scroll_offset {
                            scroll_offset = cursor_pos;
                        }
                        let _ = draw(&selected, cursor_pos, scroll_offset, &mut out);
                    }
                }
                KeyCode::Char('a') => {
                    for (i, e) in entries.iter().enumerate() {
                        if e.staleness_score.unwrap_or(0.0) > 0.0 {
                            if e.active == Some(true) && !include_active {
                                continue;
                            }
                            selected[i] = true;
                        }
                    }
                    let _ = draw(&selected, cursor_pos, scroll_offset, &mut out);
                }
                KeyCode::Char('n') => {
                    for s in selected.iter_mut() {
                        *s = false;
                    }
                    let _ = draw(&selected, cursor_pos, scroll_offset, &mut out);
                }
                _ => {}
            }
        }
    };

    let _ = terminal::disable_raw_mode();

    // Move cursor past our drawing area so subsequent output is clean
    let total_lines = visible + 2;
    let _ = execute!(out, cursor::MoveDown(total_lines as u16), Print("\n"));

    result
}

#[cfg(not(feature = "tui"))]
pub fn run_picker(
    _entries: &[crate::scan::detector::BloatEntry],
    _include_active: bool,
) -> Vec<crate::scan::detector::BloatEntry> {
    eprintln!("Interactive picker requires tui feature. Use --auto.");
    Vec::new()
}
