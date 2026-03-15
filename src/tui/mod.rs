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

/// Restores the terminal on drop, even if the TUI panics or errors.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = std::io::stdout().execute(LeaveAlternateScreen);
    }
}

/// Launch the interactive TUI for browsing audit results.
pub fn run_interactive(result: &AuditResult) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = AppState::from_result(result);

    // setup terminal — guard ensures cleanup even on early error or panic
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // main loop
    let run_result = run_loop(&mut terminal, &mut state);

    // guard handles cleanup on drop, but we still propagate any error
    drop(_guard);
    run_result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| {
            views::render(frame, state);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
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

    Ok(())
}
