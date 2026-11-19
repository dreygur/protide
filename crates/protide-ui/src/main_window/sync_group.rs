use super::*;
use gpui::{Context, Window};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::DialogFooter;
use gpui_component::{Sizable, WindowExt};

const RESET_WARNING: &str = "Every peer paired with this device drops out of sync and has to \
    pair again.";

impl MainWindow {
    /// Move to a new sync group of our own and keep it in the keychain, so
    /// every peer paired before is revoked.
    fn reset_sync_group(&mut self, cx: &mut Context<Self>) {
        let Some(secret) = self.sync_engine.as_mut().and_then(|e| e.reset_group()) else {
            return;
        };
        crate::session::save_group_secret(&secret);
        self.presence.reset_connection();
        self.console_panel.update(cx, |panel, cx| {
            panel.log(
                ConsoleEntry::team("Sync group reset: paired peers must pair again".to_string()),
                cx,
            );
        });
        cx.notify();
    }

    /// Ask before resetting the sync group; Cancel and Escape do nothing.
    fn open_reset_group_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let this = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let this = this.clone();
            alert
                .title("Reset sync group?")
                .description(RESET_WARNING)
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("reset-group-cancel")
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("reset-group-confirm")
                                .label("Reset")
                                .danger()
                                .on_click(move |_, window, cx| {
                                    this.update(cx, |this, cx| this.reset_sync_group(cx)).ok();
                                    window.close_dialog(cx);
                                }),
                        ),
                )
        });
    }

    /// The flyout's "Reset sync group" button.
    pub(super) fn render_reset_group_button(&self, cx: &mut Context<Self>) -> Button {
        Button::new("reset-sync-group")
            .label("Reset sync group")
            .small()
            .ghost()
            .on_click(cx.listener(|this, _, window, cx| this.open_reset_group_dialog(window, cx)))
    }
}
