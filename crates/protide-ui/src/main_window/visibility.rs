use super::MainWindow;
use crate::panels::ExplorerAction;
use gpui::{Context, ParentElement, Window};
use gpui_component::WindowExt;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::DialogFooter;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

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

    /// Ask what to do with unsaved edits before running `load`.
    pub fn confirm_discard(&mut self, load: ExplorerAction, cx: &mut Context<Self>) {
        self.pending_switch = Some(load);
        cx.notify();
    }

    /// Show the Save / Discard / Cancel prompt queued by `confirm_discard`.
    /// Escape and a click outside cancel, like the Cancel button.
    pub(super) fn open_discard_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(load) = self.pending_switch.take() else {
            return;
        };
        // The dialog builder and its buttons are `Fn`; the load runs once.
        let load = Rc::new(RefCell::new(Some(load)));
        let explorer = self.explorer.downgrade();
        let request_panel = self.request_panel.downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let switch = {
                let (explorer, request_panel, load) =
                    (explorer.clone(), request_panel.clone(), load.clone());
                move |save: bool, window: &mut Window, cx: &mut gpui::App| {
                    let saved = !save
                        || request_panel
                            .update(cx, |p, cx| p.save_request(cx))
                            .unwrap_or(false);
                    if saved && let Some(load) = load.borrow_mut().take() {
                        explorer.update(cx, |e, cx| load(e, cx)).ok();
                    }
                    window.close_dialog(cx);
                }
            };
            let discard = switch.clone();
            alert
                .title("Unsaved changes")
                .description("The open request has unsaved changes.")
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("discard-cancel")
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("discard-discard")
                                .label("Discard")
                                .danger()
                                .on_click(move |_, window, cx| discard(false, window, cx)),
                        )
                        .child(
                            Button::new("discard-save")
                                .label("Save")
                                .primary()
                                .on_click(move |_, window, cx| switch(true, window, cx)),
                        ),
                )
        });
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
