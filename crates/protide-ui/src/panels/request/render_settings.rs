use gpui::{Context, IntoElement, MouseButton, ParentElement, Styled, div, prelude::*, px};

use super::RequestPanel;
use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;

/// Wide enough that the Browser Profile hint stays on one line.
const LABEL_W: f32 = 220.0;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_settings_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let verify_ssl = self.verify_ssl;
        let impersonate = self.impersonate_browser;

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
                            .w(px(LABEL_W))
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
                            .rounded(px(2.0))
                            .overflow_hidden()
                            .child(
                                gpui_component::input::Input::new(&self.timeout_input)
                                    .bordered(false),
                            ),
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
                            .w(px(LABEL_W))
                            .text_size(px(13.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Verify SSL Certificate"),
                    )
                    .child(
                        self.render_kv_checkbox("ssl-toggle".into(), verify_ssl, cx)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.verify_ssl = !this.verify_ssl;
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child(if verify_ssl {
                                "enabled"
                            } else {
                                "disabled - unsafe"
                            }),
                    ),
            )
            // Browser impersonation row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .w(px(LABEL_W))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child("Browser Profile"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted.opacity(0.7))
                                    .child("Chrome 131 headers + sec-fetch-*"),
                            ),
                    )
                    .child(
                        self.render_kv_checkbox("impersonate-toggle".into(), impersonate, cx)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.impersonate_browser = !this.impersonate_browser;
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child(if impersonate {
                                "enabled — UA + sec-ch-ua + sec-fetch"
                            } else {
                                "disabled"
                            }),
                    ),
            )
            .into_any_element()
    }
}
