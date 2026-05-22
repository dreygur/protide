//! URL bar rendering for RequestPanel


use gpui::{
    canvas, div, prelude::*, px, Context, IntoElement, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Styled, Window, KeyDownEvent,
};

use crate::theme;
use crate::components::{render_text_view_with_max_scrolled, toolbar_btn};
use crate::components::icons::{
    icon, ICON_SM, ICON_MD,
    ICON_CHEVRON_DOWN, ICON_CHEVRON_UP,
    ICON_ARROW_DOWN, ICON_CODE,
};
use protide_core::execution::ws::{WebSocketExecutor};
use super::super::request_types::{RequestMode, SioConnectionState, WsConnectionState};
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_url_bar(&mut self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let method = self.method.clone();
        let method_color = theme.method_color(method.as_str());
        let is_url_focused = self.url_focus.is_focused(window);

        div()
            .w_full()
            .h(px(64.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(20.0))
            .bg(theme.colors.bg_primary)
            .border_b_1()
            .border_color(theme.colors.border.opacity(0.5))
            // Compact mode selector dropdown
            .child(
                div()
                    .id("mode-selector")
                    .w(px(110.0))
                    .h(px(32.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .bg(theme.colors.bg_secondary)
                    .border_1()
                    .border_color(if self.mode_dropdown_open {
                        theme.colors.accent
                    } else {
                        theme.colors.border
                    })
                    .cursor_pointer()
                    .when(!self.mode_dropdown_open, |s| {
                        s.hover(|s| s.border_color(theme.colors.border_focused))
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.mode_dropdown_open = !this.mode_dropdown_open;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.text_primary)
                            .child(match self.request_mode {
                                RequestMode::Http => "HTTP",
                                RequestMode::GraphQL => "GraphQL",
                                RequestMode::WebSocket => "WebSocket",
                                RequestMode::Grpc => "gRPC",
                                RequestMode::Trpc => "tRPC",
                                RequestMode::SocketIo => "Socket.IO",
                            })
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(icon(
                                if self.mode_dropdown_open { ICON_CHEVRON_UP } else { ICON_CHEVRON_DOWN },
                                ICON_SM,
                                if self.mode_dropdown_open { theme.colors.accent } else { theme.colors.text_muted },
                            ))
                    )
            )
            // Method selector button (dropdown rendered separately as overlay) - only show for HTTP mode
            .when(self.request_mode == RequestMode::Http, |el| {
                el.child(
                    div()
                        .id("method-selector")
                        .min_w(px(72.0))
                        .h(px(32.0))
                        .px(px(12.0))
                        .bg(method_color.opacity(0.1))
                        .border_1()
                        .border_color(method_color.opacity(0.2))
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(4.0))
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(method_color)
                        .cursor_pointer()
                        .hover(|s| s.bg(method_color.opacity(0.15)))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.skip_blur = true;
                            this.toggle_method_dropdown(cx);
                        }))
                        .child(method.as_str().to_string())
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(icon(ICON_CHEVRON_DOWN, ICON_SM, method_color.opacity(0.7)))
                        )
                )
            })
            // URL input with selection support
            .child(
                div()
                    .id("url-input")
                    .flex_1()
                    .min_w(px(0.0))
                    .h(px(32.0))
                    .px(px(14.0))
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .border_1()
                    .when(is_url_focused, |el| {
                        el.border_color(theme.colors.accent)
                            .bg(theme.colors.bg_primary)
                    })
                    .when(!is_url_focused, |el| {
                        el.border_color(theme.colors.border)
                            .bg(theme.colors.bg_tertiary)
                    })
                    .cursor_text()
                    .track_focus(&self.url_focus)
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            this.skip_blur = true;
                            this.focus_url(window, cx);
                            this.handle_url_mouse_down(event, cx);
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                        this.handle_url_mouse_move(event, cx);
                    }))
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &MouseUpEvent, _, cx| {
                            this.handle_url_mouse_up(event, cx);
                        }),
                    )
                    .on_mouse_up_out(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &MouseUpEvent, _, cx| {
                            this.handle_url_mouse_up(event, cx);
                        }),
                    )
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        this.handle_url_key(event, cx);
                    }))
                    // Capture this div's bounds so url_input_left = text content start in window coords
                    .child({
                        let entity = cx.entity();
                        canvas(
                            move |bounds, _, cx| {
                                let _ = entity.update(cx, |this, _| {
                                    this.url_input_left = f32::from(bounds.origin.x) + 14.0;
                                    this.url_input_width = f32::from(bounds.size.width);
                                });
                            },
                            |_, _, _, _| {},
                        )
                        .absolute().top_0().left_0().size_full()
                    })
                    .child(self.render_url_text(is_url_focused, cx)),
            )
            // Send button
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child({
                        let is_loading = self.loading;
                        let ws_connecting = self.request_mode == RequestMode::WebSocket
                            && matches!(self.ws_state, WsConnectionState::Connecting);
                        let sio_connecting = self.request_mode == RequestMode::SocketIo
                            && matches!(self.sio_state, SioConnectionState::Connecting);
                        let is_blocked = is_loading || ws_connecting || sio_connecting;

                        let (send_label, btn_bg) = match self.request_mode {
                            RequestMode::WebSocket => match self.ws_state {
                                WsConnectionState::Connected =>
                                    ("Disconnect", theme.colors.method_delete),
                                WsConnectionState::Connecting =>
                                    ("Connecting...", theme.colors.text_muted),
                                _ =>
                                    ("Connect", theme.colors.accent),
                            },
                            RequestMode::SocketIo => match self.sio_state {
                                SioConnectionState::Connected =>
                                    ("Disconnect", theme.colors.method_delete),
                                SioConnectionState::Connecting =>
                                    ("Connecting...", theme.colors.text_muted),
                                _ =>
                                    ("Connect", theme.colors.accent),
                            },
                            _ => ("Send", theme.colors.accent),
                        };
                        div()
                            .id("send-button")
                            .h(px(32.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .bg(btn_bg.opacity(if is_blocked { 0.5 } else { 1.0 }))
                            .text_color(theme.colors.bg_primary)
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .when(!is_blocked, |el| el.hover(|s| s.opacity(0.85)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.loading {
                                    return;
                                }
                                match this.request_mode {
                                    RequestMode::Http | RequestMode::GraphQL => this.send_request(cx),
                                    RequestMode::WebSocket => {
                                        if matches!(this.ws_state, WsConnectionState::Connecting) {
                                            return;
                                        }
                                        if matches!(this.ws_state, WsConnectionState::Connected) {
                                            this.disconnect_websocket(cx);
                                        } else {
                                            this.connect_websocket(cx);
                                        }
                                    }
                                    RequestMode::SocketIo => {
                                        if matches!(this.sio_state, SioConnectionState::Connecting) {
                                            return;
                                        }
                                        if matches!(this.sio_state, SioConnectionState::Connected) {
                                            this.disconnect_socketio(cx);
                                        } else {
                                            this.connect_socketio(cx);
                                        }
                                    }
                                    RequestMode::Grpc => this.send_grpc_request(cx),
                                    RequestMode::Trpc => this.send_trpc_request(cx),
                                }
                            }))
                            .child(send_label)
                    })
                    // Save button
                    .child({
                        let is_saved = self.save_feedback;
                        toolbar_btn("save-button", cx)
                            .when(is_saved, |el| el
                                .text_color(theme.colors.status_success)
                                .border_color(theme.colors.status_success.opacity(0.4))
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.save_request(cx);
                            }))
                            .child(if is_saved { "Saved!" } else { "Save" })
                    })
                    // Code generation button
                    .child(
                        toolbar_btn("code-button", cx)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.generate_code(this.codegen_language, window, cx);
                            }))
                            .child(icon(ICON_CODE, ICON_MD, theme.colors.text_secondary))
                            .child("Code")
                    )
                    // Import button
                    .child(
                        toolbar_btn("import-button", cx)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_import_modal(window, cx);
                            }))
                            .child(icon(ICON_ARROW_DOWN, ICON_SM, theme.colors.text_secondary))
                            .child("Import")
                    )
            )
    }

    pub(super) fn render_url_text(&self, is_focused: bool, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let scroll = if is_focused { self.url_scroll_offset } else { 0.0 };
        render_text_view_with_max_scrolled(
            &self.url,
            &self.url_selection,
            is_focused,
            13.0,
            theme.colors.text_primary,
            Some("Enter request URL..."),
            theme.colors.text_muted,
            None,
            theme.colors.accent.opacity(0.25),
            scroll,
        )
    }
}
