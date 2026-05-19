//! Socket.IO free-function helpers (status bar + event item)

use gpui::{div, prelude::*, px, ParentElement, SharedString, Styled};

use crate::components::icons::{icon, ICON_SM, ICON_MD, ICON_PLAY, ICON_ARROW_LEFT, ICON_ARROW_RIGHT};
use protide_core::execution::ws::WebSocketExecutor;
use protide_core::execution::sio::{SioDirection, SioEvent};
use super::super::request_types::SioConnectionState;
use super::RequestPanel;

pub(super) fn render_sio_status_bar<E: WebSocketExecutor>(
    theme: &crate::theme::Theme,
    sio_state: SioConnectionState,
    is_connected: bool,
    is_connecting: bool,
    cx: &mut gpui::Context<RequestPanel<E>>,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(4.0))
        .child(
            div()
                .flex()
                .items_center()
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
                        .child("Socket.IO")
                )
                .child(
                    div()
                        .px(px(8.0))
                        .py(px(2.0))
                        .bg(if is_connected {
                            theme.colors.method_get.opacity(0.15)
                        } else if is_connecting {
                            theme.colors.accent.opacity(0.15)
                        } else {
                            theme.colors.text_muted.opacity(0.15)
                        })
                        .text_size(px(10.0))
                        .text_color(if is_connected {
                            theme.colors.method_get
                        } else if is_connecting {
                            theme.colors.accent
                        } else {
                            theme.colors.text_muted
                        })
                        .child(match sio_state {
                            SioConnectionState::Connected => "Connected",
                            SioConnectionState::Connecting => "Connecting...",
                            SioConnectionState::Disconnected => "Disconnected",
                        })
                )
        )
        .child(
            div()
                .id("sio-connect-btn")
                .px(px(12.0))
                .py(px(6.0))
                .cursor_pointer()
                .when(is_connected, |el| {
                    el.bg(theme.colors.method_delete.opacity(0.15))
                        .text_color(theme.colors.method_delete)
                })
                .when(!is_connected, |el| {
                    el.bg(theme.colors.method_get.opacity(0.15))
                        .text_color(theme.colors.method_get)
                })
                .when(is_connecting, |el| el.cursor(gpui::CursorStyle::Arrow).opacity(0.5))
                .hover(|s| s.opacity(0.8))
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if is_connecting { return; }
                    if is_connected {
                        this.disconnect_socketio(cx);
                    } else {
                        this.connect_socketio(cx);
                    }
                }))
                .child(if is_connected { "Disconnect" } else { "Connect" })
        )
}

pub(super) fn render_sio_event_item(i: usize, event: &SioEvent, theme: &crate::theme::Theme) -> gpui::Stateful<gpui::Div> {
    let is_sent = event.direction == SioDirection::Sent;
    let is_ack = event.is_ack;
    let time_str = event.timestamp.format("%H:%M:%S").to_string();
    let ack_label = event.ack_id.map(|id| format!(" ack#{}", id)).unwrap_or_default();

    div()
        .id(SharedString::from(format!("sio-event-{}", i)))
        .w_full()
        .p(px(8.0))
        .bg(if is_sent {
            theme.colors.accent.opacity(0.08)
        } else if is_ack {
            theme.colors.method_post.opacity(0.08)
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
                        } else if is_ack {
                            theme.colors.method_post
                        } else {
                            theme.colors.method_get
                        })
                        .child(icon(
                            if is_sent { ICON_ARROW_RIGHT } else { ICON_ARROW_LEFT },
                            ICON_SM,
                            if is_sent { theme.colors.accent } else { theme.colors.method_get },
                        ))
                        .child(format!("{}{}", event.event_name, ack_label))
                )
                .child(
                    div()
                        .text_size(px(9.0))
                        .text_color(theme.colors.text_muted)
                        .child(time_str)
                )
                .child(
                    div()
                        .text_size(px(9.0))
                        .text_color(theme.colors.text_muted)
                        .child(format!("ns:{}", event.namespace))
                )
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(theme.colors.text_primary)
                .font_family("JetBrains Mono")
                .child(event.payload.clone())
        )
}
