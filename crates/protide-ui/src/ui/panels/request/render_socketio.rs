//! Socket.IO events tab rendering for RequestPanel

use std::ops::Range;

use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, Styled,
};

use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;
use protide_core::execution::sio::SioEvent;
use super::super::request_types::{EditTarget, SioConnectionState};
use super::RequestPanel;
use super::render_socketio_helpers::{render_sio_status_bar, render_sio_event_item};

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_socketio_events_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let sio_state = self.sio_state;
        let is_connected = sio_state == SioConnectionState::Connected;
        let is_connecting = sio_state == SioConnectionState::Connecting;
        let messages = self.sio_messages.clone();

        let ns_editing = self.active_edit == Some(EditTarget::SioNamespace);
        let name_editing = self.active_edit == Some(EditTarget::SioEventName);
        let ns = self.sio_namespace.clone();
        let event_name = self.sio_event_name.clone();
        let want_ack = self.sio_want_ack;
        let edit_sel = self.edit_selection.clone();

        div()
            .id("sio-events-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .track_focus(&self.edit_focus)
            // Status bar
            .child(render_sio_status_bar(&theme, sio_state, is_connected, is_connecting, cx))
            // Namespace + event name row
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .w(px(140.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("NAMESPACE")
                            )
                            .child(self.render_kv_input_flex(
                                "sio-namespace-input".to_string(),
                                EditTarget::SioNamespace,
                                &ns,
                                "/",
                                ns_editing,
                                if ns_editing { edit_sel.clone() } else { 0..0 },
                                cx,
                            ))
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("EVENT NAME")
                            )
                            .child(self.render_kv_input_flex(
                                "sio-event-name-input".to_string(),
                                EditTarget::SioEventName,
                                &event_name,
                                "message",
                                name_editing,
                                if name_editing { edit_sel } else { 0..0 },
                                cx,
                            ))
                    )
            )
            // Event history
            .child(
                div()
                    .id("sio-messages-container")
                    .flex_1()
                    .w_full()
                    .min_h(px(100.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_primary)
                    .overflow_scroll()
                    .flex()
                    .flex_col()
                    .when(messages.is_empty(), |el| {
                        el.items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("No events yet. Connect to start.")
                            )
                    })
                    .when(!messages.is_empty(), |el| {
                        el.p(px(8.0))
                            .gap(px(4.0))
                            .children(messages.iter().enumerate().map(|(i, event)| {
                                render_sio_event_item(i, event, &theme)
                            }))
                    })
            )
            // Emit composer
            .child(self.render_socketio_compose(want_ack, is_connected, cx))
            .into_any_element()
    }

    fn render_socketio_compose(&mut self, want_ack: bool, is_connected: bool, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child("PAYLOAD (JSON)")
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
                                    .when(want_ack, |el| el.child(
                                        div()
                                            .text_size(px(9.0))
                                            .text_color(theme.colors.bg_primary)
                                            .child("✓")
                                    ))
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("Request ACK")
                            )
                    )
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
                            .child(self.sio_payload_editor.clone())
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
                            .child("Emit")
                    )
            )
    }
}
