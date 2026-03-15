//! TUI view rendering for audit results.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::util::format_bytes;

use super::state::{AppState, View};

pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tab bar
            Constraint::Min(0),    // main content
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    render_tab_bar(frame, chunks[0], state);

    match state.view {
        View::Category => render_category_view(frame, chunks[1], state),
        View::Hogs => render_hogs_view(frame, chunks[1], state),
        View::Tree => render_tree_view(frame, chunks[1], state),
    }

    render_status_bar(frame, chunks[2], state);

    if state.show_help {
        render_help_overlay(frame);
    }
}

fn render_tab_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let tabs = [View::Category, View::Hogs, View::Tree];
    let spans: Vec<Span> = tabs
        .iter()
        .flat_map(|v| {
            let style = if *v == state.view {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            vec![
                Span::styled(format!(" {} ", v.label()), style),
                Span::raw(" "),
            ]
        })
        .collect();

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_category_view(frame: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .categories
        .iter()
        .enumerate()
        .map(|(i, (cat, size, pct))| {
            let style = if i == state.selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            let bar_width = (pct * 0.3).min(30.0) as usize;
            let bar: String = "█".repeat(bar_width);

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<20}", cat.label()), style),
                Span::styled(format!("{:>10}", format_bytes(*size)), style),
                Span::styled(format!(" {pct:>5.1}% "), style),
                Span::styled(bar, Style::default().fg(Color::Cyan)),
            ]))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
        " Disk Usage by Category ({}) ",
        format_bytes(state.total_bytes)
    )));

    frame.render_widget(list, area);
}

fn render_hogs_view(frame: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .top_dirs
        .iter()
        .enumerate()
        .map(|(i, (path, size, cat))| {
            let style = if i == state.selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            let path_str = path.to_string_lossy();
            let display_path = if path_str.len() > 50 {
                format!("...{}", &path_str[path_str.len() - 47..])
            } else {
                path_str.to_string()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:>3}. ", i + 1), style),
                Span::styled(format!("{display_path:<50}"), style),
                Span::styled(format!("{:>10}", format_bytes(*size)), style),
                Span::styled(
                    format!("  [{}]", cat.label()),
                    Style::default().fg(Color::Yellow),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Top Directories by Size "),
    );

    frame.render_widget(list, area);
}

fn render_tree_view(frame: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .tree_entries
        .iter()
        .enumerate()
        .map(|(i, (path, size, cat))| {
            let style = if i == state.selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            let path_str = path.to_string_lossy();

            ListItem::new(Line::from(vec![
                Span::styled(format!("{path_str:<55}"), style),
                Span::styled(format!("{:>10}", format_bytes(*size)), style),
                Span::styled(
                    format!("  {}", cat.label()),
                    Style::default().fg(Color::Green),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Directory Tree "),
    );

    frame.render_widget(list, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, _state: &AppState) {
    let status = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(Color::Cyan)),
        Span::raw(":switch "),
        Span::styled("j/k", Style::default().fg(Color::Cyan)),
        Span::raw(":nav "),
        Span::styled("?", Style::default().fg(Color::Cyan)),
        Span::raw(":help "),
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(":quit"),
    ]));

    frame.render_widget(status, area);
}

fn render_help_overlay(frame: &mut Frame) {
    let area = centered_rect(50, 60, frame.area());

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(" j / Down    Move down"),
        Line::from(" k / Up      Move up"),
        Line::from(" Tab         Switch view"),
        Line::from(" ?           Toggle help"),
        Line::from(" q           Quit"),
        Line::from(""),
        Line::from(Span::styled(
            " Views",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(" Categories  Disk usage by category"),
        Line::from(" Top Hogs    Largest directories"),
        Line::from(" Tree        Directory listing"),
        Line::from(""),
    ];

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .style(Style::default().bg(Color::Black)),
    );

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(help, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
