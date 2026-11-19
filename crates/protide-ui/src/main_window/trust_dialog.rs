use super::MainWindow;
use crate::theme;
use gpui::{App, Context, ParentElement, SharedString, Styled, Window, div, prelude::*, px};
use gpui_component::WindowExt;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::DialogFooter;

const WARNING: &str = "This request runs a script you have not trusted. It may come from an \
    imported collection or a shared folder. Read it before running it.";

impl MainWindow {
    /// Show the untrusted scripts of a send the request panel held back,
    /// with Run and trust / Cancel. Every other way of closing it, such as
    /// Escape, cancels the send just as Cancel does.
    pub(super) fn open_trust_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(scripts) = self.request_panel.update(cx, |p, _| p.trust_prompt()) else {
            return;
        };
        let text = SharedString::from(scripts.join("\n\n"));
        let panel = self.request_panel.downgrade();
        window.open_alert_dialog(cx, move |alert, _, cx| {
            let colors = theme::current(cx).colors;
            let (run, weak) = (panel.clone(), panel.clone());
            let cancel = move |cx: &mut App| {
                weak.update(cx, |p, cx| p.resolve_trust(false, cx)).ok();
            };
            let on_close = cancel.clone();
            alert
                .on_close(move |_, _, cx| on_close(cx))
                .width(px(640.0))
                .title("Run untrusted script?")
                .description(
                    div().flex().flex_col().gap_2().child(WARNING).child(
                        div()
                            .id("untrusted-script")
                            .w_full()
                            .max_h(px(320.0))
                            .overflow_scroll()
                            .p_2()
                            .rounded_md()
                            .bg(colors.bg_secondary)
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .child(text.clone()),
                    ),
                )
                .footer(
                    DialogFooter::new()
                        .child(Button::new("trust-cancel").label("Cancel").on_click(
                            move |_, window, cx| {
                                cancel(cx);
                                window.close_dialog(cx);
                            },
                        ))
                        .child(
                            Button::new("trust-run")
                                .label("Run and trust")
                                .primary()
                                .on_click(move |_, window, cx| {
                                    run.update(cx, |p, cx| p.resolve_trust(true, cx)).ok();
                                    window.close_dialog(cx);
                                }),
                        ),
                )
        });
    }
}
