//! WebSocket messages tab rendering for RequestPanel


use gpui::{
    div, prelude::*, px, Context, IntoElement, MouseDownEvent, MouseMoveEvent,
    ParentElement, SharedString, Styled,
};

use crate::theme;
use crate::components::icons::{
    icon, ICON_SM, ICON_MD,
    ICON_PLAY, ICON_ARROW_LEFT, ICON_ARROW_RIGHT,
};
use protide_core::execution::ws::{WsDirection, WebSocketExecutor};
use super::super::request_types::WsConnectionState;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_websocket_messages_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let ws_state = self.ws_state;
        let is_connected = ws_state == WsConnectionState::Connected;
        let messages = self.ws_messages.clone();
        let compose_h = self.ws_compose_h;
        let is_dragging = self.ws_compose_drag.is_some();

        let (badge_label, badge_fg, badge_bg) = match ws_state {
            WsConnectionState::Connected =>
                ("CONNECTED", theme.colors.method_get, theme.colors.method_get.opacity(0.12)),
            WsConnectionState::Connecting =>
                ("CONNECTING", theme.colors.accent, theme.colors.accent.opacity(0.12)),
            WsConnectionState::Error =>
                ("ERROR", theme.colors.error, theme.colors.error.opacity(0.12)),
            WsConnectionState::Disconnected =>
                ("DISCONNECTED", theme.colors.text_muted, theme.colors.text_muted.opacity(0.12)),
        };

        let send_bg = if is_connected { theme.colors.accent } else { theme.colors.text_muted.opacity(0.15) };
        let send_fg = if is_connected { theme.colors.bg_primary } else { theme.colors.text_muted };
        let ws_scroll = self.ws_scroll.clone();

        div()
            .id("websocket-messages-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if let Some((start_y, start_h)) = this.ws_compose_drag {
                    let delta = f32::from(event.position.y) - start_y;
                    this.ws_compose_h = (start_h - delta).clamp(60.0, 500.0);
                    cx.notify();
                }
            }))
            .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                if this.ws_compose_drag.take().is_some() {
                    cx.notify();
                }
            }))
            // Connection status row
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(4.0))
                    .pt(px(8.0))
                    .pb(px(4.0))
                    .gap(px(8.0))
                    .child(
                        div()
                            .size(px(20.0))
                            .bg(if is_connected {
                                theme.colors.method_get.opacity(0.15)
                            } else {
                                theme.colors.text_muted.opacity(0.15)
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(
                                ICON_PLAY,
                                ICON_MD,
                                if is_connected { theme.colors.method_get } else { theme.colors.text_muted },
                            ))
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("WebSocket")
                    )
                    .child(
                        div()
                            .px(px(6.0))
                            .py(px(2.0))
                            .bg(badge_bg)
                            .border_1()
                            .border_color(badge_fg.opacity(0.3))
                            .text_size(px(9.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(badge_fg)
                            .child(badge_label)
                    )
            )
            // Message history
            .child(
                div()
                    .id("ws-messages-container")
                    .flex_1()
                    .w_full()
                    .min_h(px(60.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_primary)
                    .overflow_scroll()
                    .track_scroll(&ws_scroll)
                    .flex()
                    .flex_col()
                    .when(messages.is_empty(), |el| {
                        el.items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("No messages yet. Connect to start.")
                            )
                    })
                    .when(!messages.is_empty(), |el| {
                        el.p(px(8.0))
                            .gap(px(4.0))
                            .children(messages.iter().enumerate().map(|(i, msg)| {
                                let is_sent = msg.direction == WsDirection::Sent;
                                let time_str = msg.timestamp.format("%H:%M:%S").to_string();
                                div()
                                    .id(SharedString::from(format!("ws-msg-{}", i)))
                                    .w_full()
                                    .p(px(8.0))
                                    .bg(if is_sent {
                                        theme.colors.accent.opacity(0.08)
                                    } else {
                                        theme.colors.bg_secondary
                                    })
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(4.0))
                                                    .text_size(px(10.0))
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(if is_sent {
                                                        theme.colors.accent
                                                    } else {
                                                        theme.colors.method_get
                                                    })
                                                    .child(icon(
                                                        if is_sent { ICON_ARROW_RIGHT } else { ICON_ARROW_LEFT },
                                                        ICON_SM,
                                                        if is_sent { theme.colors.accent } else { theme.colors.method_get },
                                                    ))
                                                    .child(if is_sent { "Sent" } else { "Received" })
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(9.0))
                                                    .text_color(theme.colors.text_muted)
                                                    .child(time_str)
                                            )
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_primary)
                                            .font_family("JetBrains Mono")
                                            .child(msg.content.clone())
                                    )
                            }))
                    })
            )
            // Compose resize drag handle
            .child(
                div()
                    .id("ws-compose-drag-handle")
                    .w_full()
                    .h(px(4.0))
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .cursor_row_resize()
                    .when(is_dragging, |el| el.bg(theme.colors.accent.opacity(0.3)))
                    .when(!is_dragging, |el| el.hover(|s| s.bg(theme.colors.accent.opacity(0.25))))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _, cx| {
                            this.ws_compose_drag = Some((
                                f32::from(event.position.y),
                                this.ws_compose_h,
                            ));
                            cx.notify();
                        }),
                    )
            )
            // Compose area
            .child(
                div()
                    .w_full()
                    .h(px(compose_h))
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .px(px(4.0))
                    .pt(px(6.0))
                    .pb(px(6.0))
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_muted)
                            .child("MESSAGE")
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.0))
                            .border_1()
                            .border_color(theme.colors.border)
                            .overflow_hidden()
                            .child(gpui_component::input::Input::new(&self.ws_message_editor).appearance(false))
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_end()
                            .child(
                                div()
                                    .id("ws-send-btn")
                                    .h(px(28.0))
                                    .px(px(16.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .bg(send_bg)
                                    .text_color(send_fg)
                                    .when(is_connected, |el| {
                                        el.cursor_pointer()
                                            .hover(|s| s.opacity(0.85))
                                    })
                                    .when(!is_connected, |el| el.cursor(gpui::CursorStyle::Arrow))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if is_connected {
                                            this.send_websocket_message(cx);
                                        }
                                    }))
                                    .child("Send")
                            )
                    )
            )
            .into_any_element()
    }
}
