//! Unified request/response console - tiered logging with Info, Debug, and Error levels.

mod entry;
pub use entry::*;

mod render;
mod rows;

use gpui::{Context, FocusHandle, ScrollHandle};
use std::collections::VecDeque;

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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, TestAppContext};

    fn entries(n: usize) -> Vec<ConsoleEntry> {
        (0..n)
            .map(|i| ConsoleEntry::team(format!("m{i}")))
            .collect()
    }

    #[gpui::test]
    async fn a_new_console_is_empty_with_every_filter_on(cx: &mut TestAppContext) {
        let panel = cx.new(ConsolePanel::new);
        panel.read_with(cx, |p, _| {
            assert_eq!(p.entry_count(), 0);
            assert!(p.show_team, "team events are visible by default");
            assert!(p.show_system, "system diagnostics are visible by default");
            assert!(p.context_menu.is_none());
            assert!(p.url_sel_entry.is_none());
        });
    }

    #[gpui::test]
    async fn entries_are_appended_in_arrival_order(cx: &mut TestAppContext) {
        let panel = cx.new(ConsolePanel::new);
        panel.update(cx, |p, cx| {
            for e in entries(3) {
                p.log(e, cx);
            }
        });
        panel.read_with(cx, |p, _| {
            assert_eq!(p.entry_count(), 3);
            assert_eq!(p.entries.front().unwrap().url, "m0");
            assert_eq!(p.entries.back().unwrap().url, "m2");
        });
    }

    #[gpui::test]
    async fn clearing_drops_the_entries_and_any_state_pointing_at_them(cx: &mut TestAppContext) {
        // A context menu or URL selection left behind after a clear would index
        // entries that no longer exist.
        let panel = cx.new(ConsolePanel::new);
        panel.update(cx, |p, cx| {
            for e in entries(3) {
                p.log(e, cx);
            }
            p.url_sel_entry = Some(2);
            p.context_menu = Some((2, gpui::point(gpui::px(1.0), gpui::px(1.0))));
            p.clear(cx);
        });
        panel.read_with(cx, |p, _| {
            assert_eq!(p.entry_count(), 0);
            assert!(
                p.url_sel_entry.is_none(),
                "stale selection index survived clear"
            );
            assert!(
                p.context_menu.is_none(),
                "stale context menu survived clear"
            );
        });
    }

    #[gpui::test]
    async fn the_buffer_never_grows_past_its_cap(cx: &mut TestAppContext) {
        let panel = cx.new(ConsolePanel::new);
        panel.update(cx, |p, cx| {
            for e in entries(MAX_ENTRIES * 2) {
                p.log(e, cx);
                assert!(p.entry_count() <= MAX_ENTRIES, "cap exceeded mid-run");
            }
        });
        panel.read_with(cx, |p, _| assert_eq!(p.entry_count(), MAX_ENTRIES));
    }

    #[gpui::test]
    async fn the_visibility_filters_are_independent_toggles(cx: &mut TestAppContext) {
        let panel = cx.new(ConsolePanel::new);
        panel.update(cx, |p, cx| {
            p.toggle_team(cx);
            assert!(!p.show_team);
            assert!(p.show_system, "toggling team must not affect system");
            p.toggle_system(cx);
            assert!(!p.show_system);
            p.toggle_team(cx);
            assert!(p.show_team);
            assert!(!p.show_system);
        });
    }
}
