use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*, px};

use super::RequestPanel;
use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_socketio_compose(
        &mut self,
        want_ack: bool,
        is_connected: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_muted)
                            .child("PAYLOAD (JSON)"),
                    )
                    .child(
                        div()
                            .id("sio-ack-toggle")
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.sio_want_ack = !this.sio_want_ack;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .size(px(14.0))
                                    .border_1()
                                    .border_color(if want_ack {
                                        theme.colors.accent
                                    } else {
                                        theme.colors.border
                                    })
                                    .bg(if want_ack {
                                        theme.colors.accent
                                    } else {
                                        theme.colors.bg_primary
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(want_ack, |el| {
                                        el.child(
                                            div()
                                                .text_size(px(9.0))
                                                .text_color(theme.colors.bg_primary)
                                                .child("✓"),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("Request ACK"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .h(px(80.0))
                            .border_1()
                            .border_color(theme.colors.border)
                            .overflow_hidden()
                            .child(
                                gpui_component::input::Input::new(&self.sio_payload_editor)
                                    .appearance(false)
                                    .h_full(),
                            ),
                    )
                    .child(
                        div()
                            .id("sio-emit-btn")
                            .h(px(80.0))
                            .w(px(70.0))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(is_connected, |el| {
                                el.bg(theme.colors.accent)
                                    .hover(|s| s.opacity(0.9))
                                    .text_color(theme.colors.bg_primary)
                            })
                            .when(!is_connected, |el| {
                                el.bg(theme.colors.text_muted.opacity(0.15))
                                    .cursor(gpui::CursorStyle::Arrow)
                                    .text_color(theme.colors.text_muted)
                            })
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if is_connected {
                                    this.emit_socketio_event(cx);
                                }
                            }))
                            .child("Emit"),
                    ),
            )
    }
}
