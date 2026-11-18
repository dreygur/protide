use super::MainWindow;
use gpui::Context;
use std::path::PathBuf;

impl MainWindow {
    /// Called by ExplorerPanel when the workspace root changes.
    pub(super) fn toggle_console(&mut self, cx: &mut Context<Self>) {
        self.show_console = !self.show_console;
        cx.notify();
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }

    pub fn ensure_sidebar_visible(&mut self, cx: &mut Context<Self>) {
        if self.sidebar_collapsed {
            self.sidebar_collapsed = false;
            cx.notify();
        }
    }

    pub(super) fn toggle_mock_server(&mut self, cx: &mut Context<Self>) {
        self.show_mock_server = !self.show_mock_server;
        cx.notify();
    }

    pub(super) fn toggle_docs(&mut self, cx: &mut Context<Self>) {
        self.show_docs = !self.show_docs;
        cx.notify();
    }

    pub fn show_modal(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.pending_alert = Some((title.into(), message.into(), false));
        cx.notify();
    }

    pub fn show_confirm_delete(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.pending_alert = Some((title.into(), message.into(), true));
        self.pending_delete = Some(path);
        cx.notify();
    }

    /// Close every overlay/modal. Call before opening one so at most a single
    /// overlay (Help, About, Test Runner, top menu, pairing flyout) is ever visible.
    pub(super) fn close_all_overlays(&mut self) {
        self.show_help = false;
        self.show_about = false;
        self.show_runner = false;
        self.open_menu = None;
        self.presence.show_pairing = false;
    }

    pub(super) fn dismiss_overlay(&mut self, cx: &mut Context<Self>) {
        if self.show_help {
            self.show_help = false;
        } else if self.show_about {
            self.show_about = false;
        } else if self.show_runner {
            self.show_runner = false;
        } else if self.open_menu.is_some() {
            self.open_menu = None;
        } else if self.presence.show_pairing {
            self.presence.show_pairing = false;
        }
        cx.notify();
    }
}
