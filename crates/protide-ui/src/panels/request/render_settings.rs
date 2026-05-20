use gpui::{div, prelude::*, px, Context, IntoElement, MouseButton, ParentElement, Styled};

use crate::theme;
use crate::components::icons::{icon, ICON_MD, ICON_CHECK, ICON_CLOSE};
use protide_core::execution::ws::WebSocketExecutor;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_settings_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let verify_ssl = self.verify_ssl;

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .p(px(4.0))
            // Timeout row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .w(px(160.0))
                            .text_size(px(13.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Request Timeout (s)"),
                    )
                    .child(
                        div()
                            .w(px(100.0))
                            .h(px(30.0))
                            .border_1()
                            .border_color(theme.colors.border)
                            .bg(theme.colors.bg_secondary)
                            .child(self.timeout_input.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child("seconds (0 = no timeout)"),
                    ),
            )
            // SSL verify row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .w(px(160.0))
                            .text_size(px(13.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Verify SSL Certificate"),
                    )
                    .child(
                        div()
                            .id("ssl-toggle")
                            .w(px(48.0))
                            .h(px(24.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(4.0))
                            .border_1()
                            .border_color(if verify_ssl { theme.colors.accent } else { theme.colors.border })
                            .bg(if verify_ssl {
                                theme.colors.accent.opacity(0.12)
                            } else {
                                theme.colors.bg_secondary
                            })
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                this.verify_ssl = !this.verify_ssl;
                                cx.notify();
                            }))
                            .child(icon(
                                if verify_ssl { ICON_CHECK } else { ICON_CLOSE },
                                ICON_MD,
                                if verify_ssl { theme.colors.accent } else { theme.colors.text_muted },
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child(if verify_ssl { "enabled" } else { "disabled - unsafe" }),
                    ),
            )
            .into_any_element()
    }
}
