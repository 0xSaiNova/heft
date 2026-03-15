//! Interactive TUI for the audit command.
//!
//! Feature-gated behind "tui" (ratatui + crossterm). Three views:
//! - Category: disk usage by category with proportional bars
//! - Hogs: flat sorted list of largest directories
//! - Tree: directory listing with category annotations
//!
//! Strictly read-only. No destructive operations.

mod state;
mod views;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::audit::AuditResult;
use state::AppState;

/// Launch the interactive TUI for browsing audit results.
pub fn run_interactive(result: &AuditResult) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = AppState::from_result(result);

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // main loop
    loop {
        terminal.draw(|frame| {
            views::render(frame, &state);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // ctrl+c always quits
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }

                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('j') | KeyCode::Down => state.move_down(),
                    KeyCode::Char('k') | KeyCode::Up => state.move_up(),
                    KeyCode::Tab => state.switch_view(),
                    KeyCode::Char('?') => state.show_help = !state.show_help,
                    KeyCode::Esc => {
                        if state.show_help {
                            state.show_help = false;
                        }
                    }
                    _ => {}
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    // restore terminal
    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
