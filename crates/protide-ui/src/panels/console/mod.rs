//! Unified request/response console - tiered logging with Info, Debug, and Error levels.

mod entry;
pub use entry::*;

mod rows;
mod render;

use std::collections::VecDeque;
use gpui::{Context, FocusHandle, ScrollHandle};

pub(crate) const MAX_ENTRIES: usize = 500;

pub struct ConsolePanel {
    pub(super) entries: VecDeque<ConsoleEntry>,
    pub(super) scroll: ScrollHandle,
    pub(super) focus: FocusHandle,
    /// Context-menu state: (entry index, cursor position)
    pub(super) context_menu: Option<(usize, gpui::Point<gpui::Pixels>)>,
    /// Whether to show team/sync events
    pub(super) show_team: bool,
    /// Whether to show internal P2P diagnostic events
    pub(super) show_system: bool,
    /// URL/message field of selected entry (double-click selects, Ctrl+C copies)
    pub(super) url_sel_entry: Option<usize>,
}

impl ConsolePanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_ENTRIES),
            scroll: ScrollHandle::new(),
            focus: cx.focus_handle(),
            context_menu: None,
            show_team: true,
            show_system: true,
            url_sel_entry: None,
        }
    }

    /// Append a new entry, evicting the oldest when the buffer is full.
    pub fn log(&mut self, entry: ConsoleEntry, cx: &mut Context<Self>) {
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.entries.clear();
        self.context_menu = None;
        self.url_sel_entry = None;
        cx.notify();
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn toggle_team(&mut self, cx: &mut Context<Self>) {
        self.show_team = !self.show_team;
        cx.notify();
    }

    pub fn toggle_system(&mut self, cx: &mut Context<Self>) {
        self.show_system = !self.show_system;
        cx.notify();
    }
}
