//! TUI application state and navigation.

use crate::audit::categories::AuditCategory;
use crate::audit::AuditResult;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
pub enum View {
    Category,
    Hogs,
    Tree,
}

impl View {
    pub fn next(self) -> Self {
        match self {
            View::Category => View::Hogs,
            View::Hogs => View::Tree,
            View::Tree => View::Category,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            View::Category => "Categories",
            View::Hogs => "Top Hogs",
            View::Tree => "Tree",
        }
    }
}

pub struct AppState {
    pub view: View,
    pub selected: usize,
    pub show_help: bool,
    pub should_quit: bool,

    // category view data
    pub categories: Vec<(AuditCategory, u64, f64)>, // category, size, percentage
    pub total_bytes: u64,

    // hogs view data
    pub top_dirs: Vec<(PathBuf, u64, AuditCategory)>,

    // tree view data (simplified: flat list of top dirs)
    pub tree_entries: Vec<(PathBuf, u64, AuditCategory)>,
}

impl AppState {
    pub fn from_result(result: &AuditResult) -> Self {
        let mut categories: Vec<(AuditCategory, u64, f64)> = result
            .by_category
            .iter()
            .map(|(cat, size)| {
                let pct = if result.total_bytes > 0 {
                    (*size as f64 / result.total_bytes as f64) * 100.0
                } else {
                    0.0
                };
                (*cat, *size, pct)
            })
            .collect();
        categories.sort_by(|a, b| b.1.cmp(&a.1));

        AppState {
            view: View::Category,
            selected: 0,
            show_help: false,
            should_quit: false,
            categories,
            total_bytes: result.total_bytes,
            top_dirs: result.top_dirs.clone(),
            tree_entries: result.top_dirs.clone(),
        }
    }

    pub fn item_count(&self) -> usize {
        match self.view {
            View::Category => self.categories.len(),
            View::Hogs => self.top_dirs.len(),
            View::Tree => self.tree_entries.len(),
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let count = self.item_count();
        if count > 0 && self.selected < count - 1 {
            self.selected += 1;
        }
    }

    pub fn switch_view(&mut self) {
        self.view = self.view.next();
        self.selected = 0;
    }
}
