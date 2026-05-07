//! Rendering methods for RequestPanel

use std::ops::Range;

use gpui::{
    canvas, div, prelude::*, px, Context, IntoElement, KeyDownEvent, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, SharedString, Styled, Window,
};


use crate::theme;
use crate::ui::components::{render_text_view_with_max, render_text_view_with_max_scrolled, toolbar_btn};
use crate::ui::components::icons::{
    icon, ICON_SM, ICON_MD,
    ICON_CLOSE, ICON_CHECK, ICON_CHEVRON_DOWN, ICON_CHEVRON_UP,
    ICON_CHEVRON_LEFT, ICON_CHEVRON_RIGHT, ICON_ARROW_DOWN,
    ICON_ARROW_LEFT, ICON_ARROW_RIGHT,
    ICON_FILE, ICON_FOLDER, ICON_USER, ICON_KEY,
    ICON_FORM, ICON_PLAY, ICON_CIRCLE_X, ICON_CODE,
};
use protide_core::execution::ws::{WsDirection, WebSocketExecutor};
use protide_core::execution::sio::SioDirection;
use super::super::request_types::{ApiKeyLocation, AuthType, BodyType, EditTarget, FormFieldType, GrpcStreamingType, HttpMethod, RequestMode, SioConnectionState, WsConnectionState};
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_url_bar(&mut self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let method = self.method.clone();
        let method_color = theme.method_color(method.as_str());
        let is_url_focused = self.url_focus.is_focused(window);

        // url_input_left is set each frame by the canvas() overlay inside the URL input div

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
                                    // canvas at padding edge; padding(14) to text content
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
                        let send_label = match self.request_mode {
                            RequestMode::WebSocket => if matches!(self.ws_state, WsConnectionState::Connected) {
                                "Disconnect"
                            } else {
                                "Connect"
                            },
                            RequestMode::SocketIo => if matches!(self.sio_state, SioConnectionState::Connected) {
                                "Disconnect"
                            } else {
                                "Connect"
                            },
                            _ => "Send",
                        };
                        div()
                            .id("send-button")
                            .h(px(32.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .bg(theme.colors.accent.opacity(if is_loading { 0.5 } else { 1.0 }))
                            .text_color(theme.colors.bg_primary)
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .when(!is_loading, |el| el.hover(|s| s.bg(theme.colors.accent_hover)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if !this.loading {
                                    match this.request_mode {
                                        RequestMode::Http | RequestMode::GraphQL => this.send_request(cx),
                                        RequestMode::WebSocket => {
                                            if matches!(this.ws_state, WsConnectionState::Connected) {
                                                this.disconnect_websocket(cx);
                                            } else {
                                                this.connect_websocket(cx);
                                            }
                                        }
                                        RequestMode::SocketIo => {
                                            if matches!(this.sio_state, SioConnectionState::Connected) {
                                                this.disconnect_socketio(cx);
                                            } else {
                                                this.connect_socketio(cx);
                                            }
                                        }
                                        RequestMode::Grpc => this.send_grpc_request(cx),
                                        RequestMode::Trpc => this.send_trpc_request(cx),
                                    }
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
                            .when(!is_saved, |el| el
                                .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.save_request(cx);
                            }))
                            .child(if is_saved { "Saved!" } else { "Save" })
                    })
                    // Code generation button
                    .child(
                        toolbar_btn("code-button", cx)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.generate_code(this.codegen_language, cx);
                            }))
                            .child(icon(ICON_CODE, ICON_MD, theme.colors.text_secondary))
                            .child("Code")
                    )
                    // Import button
                    .child(
                        toolbar_btn("import-button", cx)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_import_modal(cx);
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

    pub(super) fn render_method_dropdown_overlay(&mut self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        // Positioned below mode selector + gap, under the URL bar
        div()
            .id("method-dropdown-overlay")
            .absolute()
            .top(px(64.0))   // URL bar height
            .left(px(142.0)) // 20px padding + 110px mode selector + 12px gap
            .min_w(px(100.0))
            .py(px(6.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.skip_blur = true;
                cx.stop_propagation();
            }))
            .children(HttpMethod::all().iter().map(|m| {
                let m = m.clone();
                let method_color = theme.method_color(m.as_str());
                let is_selected = m == self.method;

                div()
                    .id(SharedString::from(format!("method-{}", m.as_str())))
                    .mx(px(4.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .cursor_pointer()
                    .when(is_selected, |el| {
                        el.bg(method_color.opacity(0.1))
                            .child(div().size(px(6.0)).bg(method_color))
                    })
                    .when(!is_selected, |el| {
                        el.hover(|s| s.bg(theme.colors.bg_tertiary))
                            .child(div().size(px(6.0)))
                    })
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(method_color)
                            .child(m.as_str().to_string())
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_method(m.clone(), cx);
                    }))
            }))
            // Custom method input row
            .child(
                div()
                    .id("custom-method-divider")
                    .mx(px(4.0))
                    .mt(px(4.0))
                    .border_t_1()
                    .border_color(theme.colors.border)
            )
            .child(
                div()
                    .id("custom-method-input")
                    .mx(px(4.0))
                    .px(px(12.0))
                    .my(px(4.0))
                    .h(px(28.0))
                    .bg(theme.colors.bg_primary)
                    .border_1()
                    .border_color(if self.custom_method_focus.is_focused(window) {
                        theme.colors.border_focused
                    } else {
                        theme.colors.border
                    })
                    .flex()
                    .items_center()
                    .track_focus(&self.custom_method_focus)
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| {
                        this.skip_blur = true;
                        window.focus(&this.custom_method_focus, cx);
                        cx.stop_propagation();
                    }))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        let key = event.keystroke.key.as_str();
                        match key {
                            "return" => {
                                let val = this.custom_method_input.trim().to_uppercase();
                                if !val.is_empty() {
                                    this.select_method(HttpMethod::Custom(val), cx);
                                } else {
                                    this.method_dropdown_open = false;
                                }
                                cx.notify();
                            }
                            "escape" => {
                                this.method_dropdown_open = false;
                                cx.notify();
                            }
                            "backspace" => {
                                this.custom_method_input.pop();
                                cx.notify();
                            }
                            k if k.len() == 1 && !event.keystroke.modifiers.control && !event.keystroke.modifiers.platform => {
                                this.custom_method_input.push_str(&k.to_uppercase());
                                cx.notify();
                            }
                            _ => {}
                        }
                    }))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(if self.custom_method_input.is_empty() {
                                theme.colors.text_muted
                            } else {
                                theme.colors.text_primary
                            })
                            .child(if self.custom_method_input.is_empty() {
                                "Custom method...".to_string()
                            } else {
                                self.custom_method_input.clone()
                            })
                    )
            )
    }

    pub(super) fn render_mode_dropdown_overlay(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        const MODES: &[(RequestMode, &str)] = &[
            (RequestMode::Http, "HTTP"),
            (RequestMode::GraphQL, "GraphQL"),
            (RequestMode::WebSocket, "WebSocket"),
            (RequestMode::SocketIo, "Socket.IO"),
            (RequestMode::Grpc, "gRPC"),
            (RequestMode::Trpc, "tRPC"),
        ];

        // Positioned below the mode selector button
        div()
            .id("mode-dropdown-overlay")
            .absolute()
            .top(px(68.0))  // Below URL bar (64px) + small gap
            .left(px(20.0)) // Same padding as URL bar
            .w(px(140.0))
            .py(px(6.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.skip_blur = true;
                cx.stop_propagation();
            }))
            .children(MODES.iter().map(|(mode, label)| {
                let is_selected = *mode == self.request_mode;

                div()
                    .id(SharedString::from(format!("mode-{:?}", mode)))
                    .mx(px(4.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .cursor_pointer()
                    .when(is_selected, |el| {
                        el.bg(theme.colors.accent.opacity(0.1))
                            .child(
                                div()
                                    .size(px(6.0))
                                    .bg(theme.colors.accent)
                            )
                    })
                    .when(!is_selected, |el| {
                        el.hover(|s| s.bg(theme.colors.bg_tertiary))
                            .child(div().size(px(6.0)))
                    })
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if is_selected {
                                theme.colors.text_primary
                            } else {
                                theme.colors.text_secondary
                            })
                            .child(*label)
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_request_mode(*mode, cx);
                        this.mode_dropdown_open = false;
                        cx.notify();
                    }))
            }))
    }

    pub(super) fn render_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let tab_labels: &[&str] = match self.request_mode {
            RequestMode::GraphQL => &["Query", "Variables", "Headers", "Auth"],
            RequestMode::WebSocket => &["Messages", "Headers"],
            RequestMode::SocketIo => &["Events", "Headers"],
            RequestMode::Grpc => &["Message", "Metadata", "Proto"],
            RequestMode::Trpc => &["Procedure", "Parameters", "Headers", "Auth"],
            RequestMode::Http => &["Params", "Headers", "Body", "Auth", "Scripts"],
        };
        let active_tab = self.active_tab;
        let param_count = self.params.iter().filter(|p| p.enabled && !p.key.is_empty()).count();
        let header_count = self.headers.iter().filter(|h| h.enabled && !h.key.is_empty()).count();
        let mode = self.request_mode;

        div()
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_primary)
            .children(tab_labels.iter().enumerate().map(|(i, label)| {
                let is_active = i == active_tab;
                let count = match mode {
                    RequestMode::GraphQL => if i == 2 { header_count } else { 0 },
                    RequestMode::WebSocket => if i == 1 { header_count } else { 0 },
                    RequestMode::SocketIo => if i == 1 { header_count } else { 0 },
                    RequestMode::Grpc => if i == 1 { header_count } else { 0 },
                    RequestMode::Trpc => if i == 2 { header_count } else { 0 },
                    RequestMode::Http => match i {
                        0 => param_count,
                        1 => header_count,
                        _ => 0,
                    },
                };

                div()
                    .id(SharedString::from(format!("req-tab-{}", i)))
                    .h_full()
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_pointer()
                    .border_b_2()
                    .when(is_active, |el| el.border_color(theme.colors.accent))
                    .when(!is_active, |el| {
                        el.border_color(gpui::transparent_black())
                            .hover(|s| s.bg(theme.colors.hover_overlay))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_tab(i, cx);
                    }))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(if is_active {
                                gpui::FontWeight::MEDIUM
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .text_color(if is_active {
                                theme.colors.text_primary
                            } else {
                                theme.colors.text_secondary
                            })
                            .child(*label)
                    )
                    .when(count > 0, |el| {
                        el.child(
                            div()
                                .px(px(5.0))
                                .py(px(1.0))
                                .bg(theme.colors.accent.opacity(0.15))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.accent)
                                .child(format!("{}", count))
                        )
                    })
            }))
    }

    pub(super) fn render_tab_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.request_mode {
            RequestMode::GraphQL => {
                // GraphQL tabs: Query, Variables, Headers, Auth
                match self.active_tab {
                    0 => self.render_graphql_query_tab(cx),
                    1 => self.render_graphql_variables_tab(cx),
                    2 => self.render_headers_tab(cx),
                    3 => self.render_auth_tab(cx),
                    _ => div().into_any_element(),
                }
            }
            RequestMode::WebSocket => {
                // WebSocket tabs: Messages, Headers
                match self.active_tab {
                    0 => self.render_websocket_messages_tab(cx),
                    1 => self.render_headers_tab(cx),
                    _ => div().into_any_element(),
                }
            }
            RequestMode::SocketIo => {
                // Socket.IO tabs: Events, Headers
                match self.active_tab {
                    0 => self.render_socketio_events_tab(cx),
                    1 => self.render_headers_tab(cx),
                    _ => div().into_any_element(),
                }
            }
            RequestMode::Http => {
                // HTTP tabs: Params, Headers, Body, Auth, Scripts
                match self.active_tab {
                    0 => self.render_params_tab(cx),
                    1 => self.render_headers_tab(cx),
                    2 => self.render_body_tab(cx),
                    3 => self.render_auth_tab(cx),
                    4 => self.render_scripts_tab(cx),
                    _ => div().into_any_element(),
                }
            }
            RequestMode::Grpc => {
                // gRPC tabs: Message, Metadata, Proto
                match self.active_tab {
                    0 => self.render_grpc_message_tab(cx),
                    1 => self.render_grpc_metadata_tab(cx),
                    2 => self.render_grpc_proto_tab(cx),
                    _ => div().into_any_element(),
                }
            }
            RequestMode::Trpc => {
                // tRPC tabs: Procedure, Parameters, Headers, Auth
                match self.active_tab {
                    0 => self.render_trpc_procedure_tab(cx),
                    1 => self.render_trpc_parameters_tab(cx),
                    2 => self.render_headers_tab(cx),
                    3 => self.render_auth_tab(cx),
                    _ => div().into_any_element(),
                }
            }
        }
    }

    fn render_params_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let _params_len = self.params.len();
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();
        let enabled_count = self.params.iter().filter(|p| p.enabled && !p.key.is_empty()).count();

        // Collect param data first to avoid borrow issues
        let params_data: Vec<_> = self.params.iter().enumerate().map(|(i, param)| {
            (i, param.enabled, param.key.clone(), param.value.clone())
        }).collect();

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .track_focus(&self.edit_focus);

        container = container.child(self.render_kv_table_header("KEY", "VALUE", enabled_count, cx));

        // Params list
        for (i, is_enabled, key, value) in params_data {
            let can_remove = !key.is_empty() || !value.is_empty();
            let is_editing_key = active_edit == Some(EditTarget::ParamKey(i));
            let is_editing_value = active_edit == Some(EditTarget::ParamValue(i));
            let is_row_editing = is_editing_key || is_editing_value;

            container = container.child(
                div()
                    .id(SharedString::from(format!("param-row-{}", i)))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(4.0))
                    .px(px(2.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    .child(
                        self.render_kv_checkbox(format!("param-checkbox-{}", i).into(), is_enabled, cx)
                            .on_click(cx.listener(move |this, _, _, cx| this.toggle_param(i, cx)))
                    )
                    .child(
                        self.render_kv_input(
                            format!("param-key-{}", i),
                            EditTarget::ParamKey(i),
                            &key,
                            "Key",
                            is_editing_key,
                            if is_editing_key { edit_selection.clone() } else { 0..0 },
                            px(self.kv_col_key_w),
                            cx,
                        )
                    )
                    .child(div().w(px(4.0)))
                    // Value input
                    .child(
                        self.render_kv_input_flex(
                            format!("param-value-{}", i),
                            EditTarget::ParamValue(i),
                            &value,
                            "Value",
                            is_editing_value,
                            if is_editing_value { edit_selection.clone() } else { 0..0 },
                            cx,
                        )
                    )
                    .child(
                        self.render_kv_remove_btn(format!("param-remove-{}", i).into(), can_remove, cx)
                            .when(can_remove, |el| el.on_click(cx.listener(move |this, _, _, cx| this.remove_param(i, cx))))
                    )
            );
        }

        container.into_any_element()
    }

    fn render_headers_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let _headers_len = self.headers.len();
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();
        let enabled_count = self.headers.iter().filter(|h| h.enabled && !h.key.is_empty()).count();

        // Collect header data first to avoid borrow issues
        let headers_data: Vec<_> = self.headers.iter().enumerate().map(|(i, header)| {
            (i, header.enabled, header.key.clone(), header.value.clone())
        }).collect();

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .track_focus(&self.edit_focus);

        container = container.child(self.render_kv_table_header("HEADER", "VALUE", enabled_count, cx));

        // Headers list
        for (i, is_enabled, key, value) in headers_data {
            let can_remove = !key.is_empty() || !value.is_empty();
            let is_editing_key = active_edit == Some(EditTarget::HeaderKey(i));
            let is_editing_value = active_edit == Some(EditTarget::HeaderValue(i));
            let is_row_editing = is_editing_key || is_editing_value;

            container = container.child(
                div()
                    .id(SharedString::from(format!("header-row-{}", i)))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(4.0))
                    .px(px(2.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    .child(
                        self.render_kv_checkbox(format!("header-checkbox-{}", i).into(), is_enabled, cx)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_header(i, cx);
                            }))
                    )
                    .child(
                        self.render_kv_input(
                            format!("header-key-{}", i),
                            EditTarget::HeaderKey(i),
                            &key,
                            "Header name",
                            is_editing_key,
                            if is_editing_key { edit_selection.clone() } else { 0..0 },
                            px(self.kv_col_key_w),
                            cx,
                        )
                    )
                    .child(div().w(px(4.0)))
                    .child(
                        self.render_kv_input_flex(
                            format!("header-value-{}", i),
                            EditTarget::HeaderValue(i),
                            &value,
                            "Value",
                            is_editing_value,
                            if is_editing_value { edit_selection.clone() } else { 0..0 },
                            cx,
                        )
                    )
                    .child(
                        self.render_kv_remove_btn(format!("header-remove-{}", i).into(), can_remove, cx)
                            .when(can_remove, |el| el.on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_header(i, cx);
                            })))
                    )
            );
        }

        container.into_any_element()
    }

    fn render_form_body(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let form_len = self.form_data.len();
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();
        let enabled_count = self.form_data.iter().filter(|f| f.enabled && !f.key.is_empty()).count();

        let form_data: Vec<_> = self.form_data.iter().enumerate().map(|(i, field)| {
            (i, field.enabled, field.key.clone(), field.value.clone(), field.field_type.clone(), field.file_path.is_some())
        }).collect();

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .track_focus(&self.edit_focus);

        // Toolbar row with body type buttons
        container = container.child(
            div()
                .h(px(40.0))
                .w_full()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(theme.colors.border)
                .bg(theme.colors.bg_primary)
                .child(
                    div()
                        .id("body-type-json-form")
                        .h_full()
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .border_b_2()
                        .when(self.body_type == BodyType::Json, |el| el.border_color(theme.colors.accent))
                        .when(self.body_type != BodyType::Json, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
                        .on_click(cx.listener(|this, _, _, cx| { this.set_body_type(BodyType::Json, cx); }))
                        .child(div().text_size(px(13.0))
                            .font_weight(if self.body_type == BodyType::Json { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                            .text_color(if self.body_type == BodyType::Json { theme.colors.text_primary } else { theme.colors.text_secondary })
                            .child("JSON"))
                )
                .child(
                    div()
                        .id("body-type-raw-form")
                        .h_full()
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .border_b_2()
                        .when(self.body_type == BodyType::Raw, |el| el.border_color(theme.colors.accent))
                        .when(self.body_type != BodyType::Raw, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
                        .on_click(cx.listener(|this, _, _, cx| { this.set_body_type(BodyType::Raw, cx); }))
                        .child(div().text_size(px(13.0))
                            .font_weight(if self.body_type == BodyType::Raw { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                            .text_color(if self.body_type == BodyType::Raw { theme.colors.text_primary } else { theme.colors.text_secondary })
                            .child("Raw"))
                )
                .child(
                    div()
                        .id("body-type-xml-form")
                        .h_full()
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .border_b_2()
                        .when(self.body_type == BodyType::Xml, |el| el.border_color(theme.colors.accent))
                        .when(self.body_type != BodyType::Xml, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
                        .on_click(cx.listener(|this, _, _, cx| { this.set_body_type(BodyType::Xml, cx); }))
                        .child(div().text_size(px(13.0))
                            .font_weight(if self.body_type == BodyType::Xml { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                            .text_color(if self.body_type == BodyType::Xml { theme.colors.text_primary } else { theme.colors.text_secondary })
                            .child("XML"))
                )
                .child(
                    div()
                        .id("body-type-form-form")
                        .h_full()
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .border_b_2()
                        .when(self.body_type == BodyType::Form, |el| el.border_color(theme.colors.accent))
                        .when(self.body_type != BodyType::Form, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
                        .on_click(cx.listener(|this, _, _, cx| { this.set_body_type(BodyType::Form, cx); }))
                        .child(div().text_size(px(13.0))
                            .font_weight(if self.body_type == BodyType::Form { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                            .text_color(if self.body_type == BodyType::Form { theme.colors.text_primary } else { theme.colors.text_secondary })
                            .child("Form"))
                )
                .child(
                    div()
                        .id("body-type-binary-form")
                        .h_full()
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .border_b_2()
                        .when(self.body_type == BodyType::Binary, |el| el.border_color(theme.colors.accent))
                        .when(self.body_type != BodyType::Binary, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
                        .on_click(cx.listener(|this, _, _, cx| { this.set_body_type(BodyType::Binary, cx); }))
                        .child(div().text_size(px(13.0))
                            .font_weight(if self.body_type == BodyType::Binary { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                            .text_color(if self.body_type == BodyType::Binary { theme.colors.text_primary } else { theme.colors.text_secondary })
                            .child("Binary"))
                )
                .child(div().flex_1())
                .child(
                    div()
                        .px(px(12.0))
                        .py(px(2.0))
                        .bg(theme.colors.accent.opacity(0.12))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.colors.accent)
                        .child(format!("{} fields", enabled_count))
                )
        );

        // Table header — form has an extra TYPE column so render it manually (same style as render_kv_table_header)
        container = container.child(
            div()
                .w_full().flex().items_center()
                .gap(px(8.0))
                .py(px(6.0))
                .border_b_1().border_color(theme.colors.border)
                .mb(px(4.0))
                .child(div().size(px(16.0)))
                .child(
                    div()
                        .w(px(130.0))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.accent.opacity(0.7))
                        .child("KEY")
                )
                .child(
                    div()
                        .w(px(50.0))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text_secondary)
                        .child("TYPE")
                )
                .child(
                    div()
                        .flex_1().flex().items_center().justify_between()
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.colors.text_secondary)
                                .child("VALUE")
                        )
                        .child(self.render_count_badge(enabled_count, "active", cx))
                )
                .child(div().size(px(28.0)))
        );

        // Form fields list
        for (i, is_enabled, key, value, field_type, has_file) in form_data {
            let can_remove = form_len > 1;
            let is_file_type = field_type == FormFieldType::File;
            let is_editing_key = active_edit == Some(EditTarget::FormKey(i));
            let is_editing_value = active_edit == Some(EditTarget::FormValue(i));
            let is_row_editing = is_editing_key || is_editing_value;

            container = container.child(
                div()
                    .id(SharedString::from(format!("form-row-{}", i)))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(4.0))
                    .px(px(2.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    .child(
                        self.render_kv_checkbox(format!("form-checkbox-{}", i).into(), is_enabled, cx)
                            .on_click({
                                let idx = i;
                                cx.listener(move |this, _, _, cx| {
                                    this.toggle_form_field(idx, cx);
                                })
                            })
                    )
                    // Key input
                    .child(
                        self.render_kv_input(
                            format!("form-key-{}", i),
                            EditTarget::FormKey(i),
                            &key,
                            "key",
                            is_editing_key,
                            edit_selection.clone(),
                            px(130.0),
                            cx,
                        )
                    )
                    // Type selector
                    .child(
                        div()
                            .id(SharedString::from(format!("form-type-{}", i)))
                            .w(px(50.0))
                            .h(px(24.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .border_1()
                            .border_color(theme.colors.border)
                            .bg(theme.colors.bg_tertiary)
                            .hover(|s| s.border_color(theme.colors.text_muted))
                            .text_size(px(10.0))
                            .text_color(if is_file_type { theme.colors.accent } else { theme.colors.text_muted })
                            .on_click({
                                let idx = i;
                                cx.listener(move |this, _, _, cx| {
                                    this.toggle_form_field_type(idx, cx);
                                })
                            })
                            .child(if is_file_type { "File" } else { "Text" })
                    )
                    // Value input or file picker
                    .when(!is_file_type, |el| {
                        el.child(
                            self.render_kv_input_flex(
                                format!("form-value-{}", i),
                                EditTarget::FormValue(i),
                                &value,
                                "value",
                                is_editing_value,
                                edit_selection.clone(),
                                cx,
                            )
                        )
                    })
                    .when(is_file_type, |el| {
                        el.child(
                            div()
                                .id(SharedString::from(format!("form-file-{}", i)))
                                .flex_1()
                                .h(px(24.0))
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .id(SharedString::from(format!("form-file-btn-{}", i)))
                                        .px(px(10.0))
                                        .h(px(24.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .bg(theme.colors.bg_tertiary)
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .hover(|s| s.border_color(theme.colors.accent))
                                        .text_size(px(11.0))
                                        .text_color(theme.colors.text_secondary)
                                        .on_click({
                                            let idx = i;
                                            cx.listener(move |this, _, _, cx| {
                                                this.select_form_file(idx, cx);
                                            })
                                        })
                                        .child("Choose File")
                                )
                                .when(has_file, |el| {
                                    el.child(
                                        div()
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .overflow_hidden()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_primary)
                                            .child(value.clone())
                                    )
                                })
                                .when(!has_file, |el| {
                                    el.child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("No file selected")
                                    )
                                })
                        )
                    })
                    .child(
                        self.render_kv_remove_btn(format!("form-remove-{}", i).into(), can_remove, cx)
                            .when(can_remove, |el| el.on_click({
                                let idx = i;
                                cx.listener(move |this, _, _, cx| {
                                    this.remove_form_field(idx, cx);
                                })
                            }))
                    )
            );
        }

        container = container.child(
            div().w_full().pt(px(8.0)).child(
                self.render_kv_add_btn("add-form-field-btn", "+ Add Field", cx)
                    .on_click(cx.listener(|this, _, _, cx| { this.add_form_field(cx); }))
            )
        );

        container.into_any_element()
    }

    fn render_body_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.body_type == BodyType::Form {
            return self.render_form_body(cx);
        }
        if self.body_type == BodyType::Binary {
            return self.render_binary_body(cx);
        }

        let theme = theme::current(cx);

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Toolbar row
            .child(
                div()
                    .h(px(40.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_primary)
                    .child(
                        div()
                            .id("body-type-json")
                            .h_full()
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .when(self.body_type == BodyType::Json, |el| el.border_color(theme.colors.accent))
                            .when(self.body_type != BodyType::Json, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
                            .on_click(cx.listener(|this, _, _, cx| { this.set_body_type(BodyType::Json, cx); }))
                            .child(div().text_size(px(13.0))
                                .font_weight(if self.body_type == BodyType::Json { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                                .text_color(if self.body_type == BodyType::Json { theme.colors.text_primary } else { theme.colors.text_secondary })
                                .child("JSON"))
                    )
                    .child(
                        div()
                            .id("body-type-raw")
                            .h_full()
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .when(self.body_type == BodyType::Raw, |el| el.border_color(theme.colors.accent))
                            .when(self.body_type != BodyType::Raw, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
                            .on_click(cx.listener(|this, _, _, cx| { this.set_body_type(BodyType::Raw, cx); }))
                            .child(div().text_size(px(13.0))
                                .font_weight(if self.body_type == BodyType::Raw { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                                .text_color(if self.body_type == BodyType::Raw { theme.colors.text_primary } else { theme.colors.text_secondary })
                                .child("Raw"))
                    )
                    .child(
                        div()
                            .id("body-type-xml")
                            .h_full()
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .when(self.body_type == BodyType::Xml, |el| el.border_color(theme.colors.accent))
                            .when(self.body_type != BodyType::Xml, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
                            .on_click(cx.listener(|this, _, _, cx| { this.set_body_type(BodyType::Xml, cx); }))
                            .child(div().text_size(px(13.0))
                                .font_weight(if self.body_type == BodyType::Xml { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                                .text_color(if self.body_type == BodyType::Xml { theme.colors.text_primary } else { theme.colors.text_secondary })
                                .child("XML"))
                    )
                    .child(
                        div()
                            .id("body-type-form")
                            .h_full()
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .when(self.body_type == BodyType::Form, |el| el.border_color(theme.colors.accent))
                            .when(self.body_type != BodyType::Form, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
                            .on_click(cx.listener(|this, _, _, cx| { this.set_body_type(BodyType::Form, cx); }))
                            .child(div().text_size(px(13.0))
                                .font_weight(if self.body_type == BodyType::Form { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                                .text_color(if self.body_type == BodyType::Form { theme.colors.text_primary } else { theme.colors.text_secondary })
                                .child("Form"))
                    )
                    .child(
                        div()
                            .id("body-type-binary")
                            .h_full()
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .when(self.body_type == BodyType::Binary, |el| el.border_color(theme.colors.accent))
                            .when(self.body_type != BodyType::Binary, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
                            .on_click(cx.listener(|this, _, _, cx| { this.set_body_type(BodyType::Binary, cx); }))
                            .child(div().text_size(px(13.0))
                                .font_weight(if self.body_type == BodyType::Binary { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                                .text_color(if self.body_type == BodyType::Binary { theme.colors.text_primary } else { theme.colors.text_secondary })
                                .child("Binary"))
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .px(px(12.0))
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("request body")
                    )
            )
            // Body editor using CodeEditor component
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(self.body_editor.clone())
            )
            .into_any_element()
    }

    fn render_binary_body(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let file_name = self.binary_file_path.as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string());
        let file_size = self.binary_file_path.as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| {
                let bytes = m.len();
                if bytes < 1024 { format!("{} B", bytes) }
                else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
                else { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
            });

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.0))
                    .when_some(file_name, |el, name| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(icon(ICON_FILE, ICON_MD, theme.colors.accent))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .text_color(theme.colors.text_primary)
                                        .child(name)
                                )
                                .when_some(file_size, |el, size| {
                                    el.child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child(size)
                                    )
                                })
                        )
                    })
                    .when(self.binary_file_path.is_none(), |el| {
                        el.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("No file selected")
                        )
                    })
                    .child(
                        div()
                            .id("browse-binary-btn")
                            .px(px(16.0))
                            .py(px(8.0))
                            .bg(theme.colors.bg_tertiary)
                            .border_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .cursor_pointer()
                            .hover(|s| s.border_color(theme.colors.accent))
                            .on_click(cx.listener(|this, _, _, cx| this.browse_binary_file(cx)))
                            .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_secondary))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child(if self.binary_file_path.is_some() { "Change File" } else { "Browse File" })
                            )
                    )
            )
            .into_any_element()
    }

    /// Drag handle between KEY and VALUE column headers in KV tables
    fn render_kv_col_drag_handle(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let start_w = self.kv_col_key_w;
        div()
            .id("kv-col-drag-handle")
            .w(px(4.0))
            .self_stretch()
            .cursor_col_resize()
            .bg(theme.colors.border.opacity(0.6))
            .hover(|s| s.bg(theme.colors.accent.opacity(0.6)))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                this.kv_col_drag = Some((f32::from(event.position.x), start_w));
                cx.notify();
            }))
    }

    fn render_count_badge(&self, count: usize, suffix: &str, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        div()
            .px(px(6.0)).py(px(2.0))
            .bg(theme.colors.accent.opacity(0.12))
            .border_1().border_color(theme.colors.accent.opacity(0.35))
            .text_size(px(10.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.colors.accent)
            .child(format!("{} {}", count, suffix))
    }

    fn render_kv_table_header(&self, key_label: &'static str, value_label: &'static str, count: usize, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        div()
            .w_full().flex().items_center()
            .gap(px(8.0))
            .py(px(6.0))
            .border_b_1().border_color(theme.colors.border)
            .mb(px(4.0))
            .child(div().size(px(16.0)))
            .child(
                div()
                    .w(px(self.kv_col_key_w))
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.colors.accent.opacity(0.7))
                    .child(key_label)
            )
            .child(self.render_kv_col_drag_handle(cx))
            .child(
                div().flex_1().flex().items_center().justify_between()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_secondary)
                            .child(value_label)
                    )
                    .child(self.render_count_badge(count, "active", cx))
            )
            .child(div().size(px(28.0)))
    }

    fn render_kv_checkbox(&self, id: SharedString, is_enabled: bool, cx: &Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = theme::current(cx);
        div()
            .id(id)
            .size(px(16.0))
            .border_1()
            .cursor_pointer()
            .when(is_enabled, |el| el.bg(theme.colors.accent).border_color(theme.colors.accent))
            .when(!is_enabled, |el| {
                el.border_color(theme.colors.border)
                    .hover(|s| s.border_color(theme.colors.text_muted))
            })
            .flex().items_center().justify_center()
            .when(is_enabled, |el| {
                el.child(div().flex().items_center().justify_center().child(icon(ICON_CHECK, ICON_SM, theme.colors.bg_primary)))
            })
    }

    fn render_kv_remove_btn(&self, id: SharedString, can_remove: bool, cx: &Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = theme::current(cx);
        div()
            .id(id)
            .size(px(28.0))
            .flex().items_center().justify_center()
            .text_size(px(14.0))
            .when(can_remove, |el| {
                el.cursor_pointer()
                    .text_color(theme.colors.text_muted)
                    .hover(|s| s.bg(theme.colors.status_client_error.opacity(0.1)).text_color(theme.colors.status_client_error))
            })
            .when(!can_remove, |el| {
                el.cursor(gpui::CursorStyle::Arrow)
                    .text_color(theme.colors.border)
            })
            .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted))
    }

    fn render_kv_add_btn(&self, id: &'static str, label: &'static str, cx: &Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = theme::current(cx);
        div()
            .id(id)
            .w_full()
            .py(px(8.0))
            .border_1()
            .border_color(theme.colors.border.opacity(0.5))
            .flex().items_center().justify_center()
            .cursor_pointer()
            .text_size(px(12.0))
            .text_color(theme.colors.text_muted)
            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.border).text_color(theme.colors.text_secondary))
            .child(label)
    }

    /// Render a key-value input field with fixed width
    fn render_kv_input(
        &mut self,
        id: String,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        width: gpui::Pixels,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(SharedString::from(id))
            .w(width)
            .min_w(px(0.0))
            // Dynamic height: fixed when unfocused, expands when focused
            .when(!is_editing, |el| el.h(px(28.0)).overflow_hidden())
            .when(is_editing, |el| el.min_h(px(28.0)).py(px(4.0)))
            .px(px(8.0))
            .flex()
            .when(!is_editing, |el| el.items_center())
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| {
                el.border_color(gpui::transparent_black())
                  .hover(|s| s.border_color(theme.colors.border))
            })
            .bg(theme.colors.bg_tertiary)
            .text_size(px(12.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.skip_blur = true;
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, target, 7.2, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 7.2, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_edit_mouse_up(event, cx);
                }),
            )
            // Canvas captures this div's origin so mouse handlers get accurate text-start position
            .child({
                let entity = cx.entity();
                canvas(
                    move |bounds, _, cx| {
                        let _ = entity.update(cx, |this, _| {
                            // canvas at padding edge; padding(8) to text content
                            this.edit_input_origins.insert(target, f32::from(bounds.origin.x) + 8.0);
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute().top_0().left_0().size_full()
            })
            // Compute chars per line from actual pixel width (char_width = 12px * 0.6 = 7.2, padding = 16px)
            .child({
                let max_chars = (((f32::from(width) - 16.0) / 7.2).max(1.0)) as usize;
                self.render_kv_text(&text, placeholder, is_editing, selection, Some(max_chars), cx)
            })
    }

    /// Render a key-value input field with flex width
    fn render_kv_input_flex(
        &mut self,
        id: String,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(SharedString::from(id))
            .flex_1()
            .min_w(px(0.0))
            // Dynamic height: fixed when unfocused, expands when focused
            .when(!is_editing, |el| el.h(px(28.0)).overflow_hidden())
            .when(is_editing, |el| el.min_h(px(28.0)).py(px(4.0)))
            .px(px(8.0))
            .flex()
            .when(!is_editing, |el| el.items_center())
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| {
                el.border_color(gpui::transparent_black())
                  .hover(|s| s.border_color(theme.colors.border))
            })
            .bg(theme.colors.bg_tertiary)
            .text_size(px(12.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.skip_blur = true;
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, target, 7.2, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 7.2, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_edit_mouse_up(event, cx);
                }),
            )
            // Canvas captures origin for accurate click-to-cursor
            .child({
                let entity = cx.entity();
                canvas(
                    move |bounds, _, cx| {
                        let _ = entity.update(cx, |this, _| {
                            this.edit_input_origins.insert(target, f32::from(bounds.origin.x) + 9.0);
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute().top_0().left_0().size_full()
            })
            // Single-line for flex value column (width is unknown at render time)
            .child(self.render_kv_text(&text, placeholder, is_editing, selection, None, cx))
    }

    /// Render text with cursor/selection for kv inputs
    fn render_kv_text(
        &self,
        text: &str,
        placeholder: &'static str,
        is_focused: bool,
        selection: Range<usize>,
        max_chars: Option<usize>,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        render_text_view_with_max(
            text,
            &selection,
            is_focused,
            12.0,
            theme.colors.text_primary,
            Some(placeholder),
            theme.colors.text_muted,
            max_chars,
            theme.colors.accent.opacity(0.25),
        )
    }

    fn render_auth_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let auth_type = self.auth_type;
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();

        let auth_types = [
            (AuthType::None, "None"),
            (AuthType::Bearer, "Bearer"),
            (AuthType::Basic, "Basic"),
            (AuthType::ApiKey, "API Key"),
        ];

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .track_focus(&self.edit_focus)
            // Auth type selector - same style as main tab bar
            .child(
                div()
                    .h(px(40.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_primary)
                    .children(auth_types.iter().map(|(at, label)| {
                        let is_selected = *at == auth_type;
                        let at = *at;
                        div()
                            .id(SharedString::from(format!("auth-type-{:?}", at)))
                            .h_full()
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .when(is_selected, |el| el.border_color(theme.colors.accent))
                            .when(!is_selected, |el| {
                                el.border_color(gpui::transparent_black())
                                    .hover(|s| s.bg(theme.colors.hover_overlay))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_auth_type(at, cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(if is_selected {
                                        gpui::FontWeight::MEDIUM
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .text_color(if is_selected {
                                        theme.colors.text_primary
                                    } else {
                                        theme.colors.text_secondary
                                    })
                                    .child(*label)
                            )
                    }))
            )
            // Auth type specific content
            .child(self.render_auth_content(auth_type, active_edit, edit_selection, cx))
            .into_any_element()
    }

    fn render_auth_content(
        &mut self,
        auth_type: AuthType,
        active_edit: Option<EditTarget>,
        edit_selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);

        match auth_type {
            AuthType::None => {
                div()
                    .w_full()
                    .p(px(16.0))
                    .bg(theme.colors.bg_tertiary.opacity(0.5))
                    .border_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .size(px(32.0))
                            .bg(theme.colors.text_muted.opacity(0.1))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(16.0))
                            .text_color(theme.colors.text_muted)
                            .child(icon(ICON_CIRCLE_X, ICON_SM, theme.colors.text_muted))
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
            .p(px(20.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("No Authentication")
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("Request will be sent without auth headers")
                            )
                    )
                    .into_any_element()
            }
            AuthType::Bearer => {
                let token = self.bearer_token.clone();
                let is_editing = active_edit == Some(EditTarget::BearerToken);

                div()
                    .w_full()
                    .p(px(16.0))
                    .bg(theme.colors.bg_tertiary.opacity(0.5))
                    .border_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    // Header
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .size(px(32.0))
                                    .bg(theme.colors.accent.opacity(0.1))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icon(ICON_FORM, ICON_MD, theme.colors.accent))
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_primary)
                                            .child("Bearer Token")
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("Authorization: Bearer <token>")
                                    )
                            )
                    )
                    // Token input
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("TOKEN")
                            )
                            .child(
                                self.render_auth_input(
                                    "bearer-token",
                                    EditTarget::BearerToken,
                                    &token,
                                    "Enter bearer token...",
                                    is_editing,
                                    if is_editing { edit_selection } else { 0..0 },
                                    cx,
                                )
                            )
                    )
                    .into_any_element()
            }
            AuthType::Basic => {
                let username = self.basic_username.clone();
                let password = self.basic_password.clone();
                let is_editing_user = active_edit == Some(EditTarget::BasicUsername);
                let is_editing_pass = active_edit == Some(EditTarget::BasicPassword);

                div()
                    .w_full()
                    .p(px(16.0))
                    .bg(theme.colors.bg_tertiary.opacity(0.5))
                    .border_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    // Header
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .size(px(32.0))
                                    .bg(theme.colors.accent.opacity(0.1))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icon(ICON_USER, ICON_MD, theme.colors.accent))
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_primary)
                                            .child("Basic Authentication")
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("Authorization: Basic <base64>")
                                    )
                            )
                    )
                    // Fields in a row
                    .child(
                        div()
                            .flex()
                            .gap(px(12.0))
                            // Username
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_secondary)
                                            .child("USERNAME")
                                    )
                                    .child(
                                        self.render_auth_input(
                                            "basic-username",
                                            EditTarget::BasicUsername,
                                            &username,
                                            "Enter username...",
                                            is_editing_user,
                                            if is_editing_user { edit_selection.clone() } else { 0..0 },
                                            cx,
                                        )
                                    )
                            )
                            // Password
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_secondary)
                                            .child("PASSWORD")
                                    )
                                    .child(
                                        self.render_auth_input_masked(
                                            "basic-password",
                                            EditTarget::BasicPassword,
                                            &password,
                                            "Enter password...",
                                            is_editing_pass,
                                            if is_editing_pass { edit_selection } else { 0..0 },
                                            cx,
                                        )
                                    )
                            )
                    )
                    .into_any_element()
            }
            AuthType::ApiKey => {
                let key_name = self.api_key_name.clone();
                let key_value = self.api_key_value.clone();
                let location = self.api_key_location;
                let is_editing_name = active_edit == Some(EditTarget::ApiKeyName);
                let is_editing_value = active_edit == Some(EditTarget::ApiKeyValue);

                div()
                    .w_full()
                    .p(px(16.0))
                    .bg(theme.colors.bg_tertiary.opacity(0.5))
                    .border_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    // Header
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .size(px(32.0))
                                    .bg(theme.colors.accent.opacity(0.1))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icon(ICON_KEY, ICON_MD, theme.colors.accent))
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_primary)
                                            .child("API Key")
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(theme.colors.text_muted)
                                            .when(location == ApiKeyLocation::Header, |el| {
                                                el.child("Sent as request header")
                                            })
                                            .when(location == ApiKeyLocation::QueryParam, |el| {
                                                el.child("Sent as query parameter")
                                            })
                                    )
                            )
                    )
                    // Fields in a row
                    .child(
                        div()
                            .flex()
                            .gap(px(12.0))
                            // Key name
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_secondary)
                                            .child("KEY NAME")
                                    )
                                    .child(
                                        self.render_auth_input(
                                            "api-key-name",
                                            EditTarget::ApiKeyName,
                                            &key_name,
                                            "e.g., X-API-Key",
                                            is_editing_name,
                                            if is_editing_name { edit_selection.clone() } else { 0..0 },
                                            cx,
                                        )
                                    )
                            )
                            // Key value
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_secondary)
                                            .child("VALUE")
                                    )
                                    .child(
                                        self.render_auth_input_masked(
                                            "api-key-value",
                                            EditTarget::ApiKeyValue,
                                            &key_value,
                                            "Enter API key...",
                                            is_editing_value,
                                            if is_editing_value { edit_selection } else { 0..0 },
                                            cx,
                                        )
                                    )
                            )
                    )
                    // Location selector
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("ADD TO")
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .p(px(3.0))
                                    .bg(theme.colors.bg_primary)
                                    .child(
                                        div()
                                            .id("api-key-header")
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .px(px(10.0))
                                            .py(px(5.0))
                                            .cursor_pointer()
                                            .text_size(px(11.0))
                                            .when(location == ApiKeyLocation::Header, |el| {
                                                el.bg(theme.colors.bg_tertiary)
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(theme.colors.text_primary)
                                                    .child("H")
                                            })
                                            .when(location != ApiKeyLocation::Header, |el| {
                                                el.text_color(theme.colors.text_muted)
                                                    .hover(|s| s.text_color(theme.colors.text_secondary))
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if this.api_key_location != ApiKeyLocation::Header {
                                                    this.toggle_api_key_location(cx);
                                                }
                                            }))
                                            .child("Header")
                                    )
                                    .child(
                                        div()
                                            .id("api-key-query")
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .px(px(10.0))
                                            .py(px(5.0))
                                            .cursor_pointer()
                                            .text_size(px(11.0))
                                            .when(location == ApiKeyLocation::QueryParam, |el| {
                                                el.bg(theme.colors.bg_tertiary)
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(theme.colors.text_primary)
                                                    .child("?")
                                            })
                                            .when(location != ApiKeyLocation::QueryParam, |el| {
                                                el.text_color(theme.colors.text_muted)
                                                    .hover(|s| s.text_color(theme.colors.text_secondary))
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if this.api_key_location != ApiKeyLocation::QueryParam {
                                                    this.toggle_api_key_location(cx);
                                                }
                                            }))
                                            .child("Query Param")
                                    )
                            )
                    )
                    .into_any_element()
            }
        }
    }

    fn render_auth_input(
        &mut self,
        id: &str,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.render_auth_input_impl(id, target, text, placeholder, is_editing, selection, false, cx)
    }

    fn render_auth_input_masked(
        &mut self,
        id: &str,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.render_auth_input_impl(id, target, text, placeholder, is_editing, selection, true, cx)
    }

    fn render_auth_input_impl(
        &mut self,
        id: &str,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        masked: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let display_text = if masked && !text.is_empty() {
            "●".repeat(text.len())
        } else {
            text.to_string()
        };

        div()
            .id(SharedString::from(id.to_string()))
            .w_full()
            .max_w(px(400.0))
            .min_w(px(0.0))
            // Dynamic height: fixed when unfocused, expands when focused
            .when(!is_editing, |el| el.h(px(32.0)).overflow_hidden())
            .when(is_editing, |el| el.min_h(px(32.0)).py(px(6.0)))
            .px(px(12.0))
            .flex()
            .when(!is_editing, |el| el.items_center())
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| {
                el.border_color(gpui::transparent_black())
                  .hover(|s| s.border_color(theme.colors.border))
            })
            .bg(theme.colors.bg_tertiary)
            .text_size(px(12.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.skip_blur = true;
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, target, 7.2, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 7.2, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_edit_mouse_up(event, cx);
                }),
            )
            .child({
                let entity = cx.entity();
                canvas(
                    move |bounds, _, cx| {
                        let _ = entity.update(cx, |this, _| {
                            // canvas at padding edge; padding(12) to text content
                            this.edit_input_origins.insert(target, f32::from(bounds.origin.x) + 12.0);
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute().top_0().left_0().size_full()
            })
            // ~50 chars for 400px max width
            .child(self.render_kv_text(&display_text, placeholder, is_editing, selection, Some(50), cx))
    }

    fn render_scripts_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let script_pre_open = self.script_pre_open;
        let script_post_open = self.script_post_open;
        let script_tests_open = self.script_tests_open;
        let script_pre_h = self.script_pre_h;
        let script_post_h = self.script_post_h;

        div()
            .id("scripts-tab")
            .w_full()
            .h_full()
            .relative()
            .flex()
            .flex_col()
            // Pre-request Script section
            .child(
                div()
                    .id("script-pre-header")
                    .h(px(36.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.5)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.script_pre_open = !this.script_pre_open;
                        cx.notify();
                    }))
                    .child(if script_pre_open {
                        icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                    } else {
                        icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                    })
                    .child(
                        div()
                            .size(px(18.0))
                            .bg(theme.colors.method_post.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.method_post))
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Pre-request Script")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Runs before sending the request")
                    )
            )
            .when(script_pre_open, |el| {
                el.child(
                    div()
                        .h(px(script_pre_h))
                        .w_full()
                        .overflow_hidden()
                        .child(self.pre_script_editor.clone()),
                )
                .child(
                    div()
                        .id("drag-script-pre")
                        .w_full()
                        .h(px(4.0))
                        .cursor_row_resize()
                        .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, event: &MouseDownEvent, _, _| {
                                this.drag_script_pre =
                                    Some((f32::from(event.position.y), this.script_pre_h));
                            }),
                        ),
                )
            })
            // Post-response Script section
            .child(
                div()
                    .id("script-post-header")
                    .h(px(36.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.5)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.script_post_open = !this.script_post_open;
                        cx.notify();
                    }))
                    .child(if script_post_open {
                        icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                    } else {
                        icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                    })
                    .child(
                        div()
                            .size(px(18.0))
                            .bg(theme.colors.status_success.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(ICON_CHEVRON_LEFT, ICON_SM, theme.colors.status_success))
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Post-response Script")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Runs after receiving response")
                    )
            )
            .when(script_post_open, |el| {
                el.child(
                    div()
                        .h(px(script_post_h))
                        .w_full()
                        .overflow_hidden()
                        .child(self.post_script_editor.clone()),
                )
                .child(
                    div()
                        .id("drag-script-post")
                        .w_full()
                        .h(px(4.0))
                        .cursor_row_resize()
                        .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, event: &MouseDownEvent, _, _| {
                                this.drag_script_post =
                                    Some((f32::from(event.position.y), this.script_post_h));
                            }),
                        ),
                )
            })
            // Tests section (flex_1 — fills remaining space)
            .child(
                div()
                    .id("script-tests-header")
                    .h(px(36.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.5)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.script_tests_open = !this.script_tests_open;
                        cx.notify();
                    }))
                    .child(if script_tests_open {
                        icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                    } else {
                        icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                    })
                    .child(
                        div()
                            .size(px(18.0))
                            .bg(theme.colors.accent.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(ICON_CHECK, ICON_SM, theme.colors.accent))
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Tests")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Test assertions using expect()")
                    )
            )
            .when(script_tests_open, |el| {
                el.child(
                    div()
                        .flex_1()
                        .w_full()
                        .overflow_hidden()
                        .child(self.tests_editor.clone()),
                )
            })
            // Pre-script drag overlay
            .when(self.drag_script_pre.is_some(), |el| {
                el.child(gpui::deferred(
                    div()
                        .id("drag-script-pre-overlay")
                        .absolute()
                        .inset_0()
                        .cursor_row_resize()
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                            if let Some((start_y, start_h)) = this.drag_script_pre {
                                let delta = f32::from(event.position.y) - start_y;
                                this.script_pre_h = (start_h + delta).max(60.0).min(600.0);
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.drag_script_pre = None;
                                crate::prefs::set_f32("request.script_pre_h", this.script_pre_h);
                                cx.notify();
                            }),
                        ),
                ).with_priority(2))
            })
            // Post-script drag overlay
            .when(self.drag_script_post.is_some(), |el| {
                el.child(gpui::deferred(
                    div()
                        .id("drag-script-post-overlay")
                        .absolute()
                        .inset_0()
                        .cursor_row_resize()
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                            if let Some((start_y, start_h)) = this.drag_script_post {
                                let delta = f32::from(event.position.y) - start_y;
                                this.script_post_h = (start_h + delta).max(60.0).min(600.0);
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.drag_script_post = None;
                                crate::prefs::set_f32("request.script_post_h", this.script_post_h);
                                cx.notify();
                            }),
                        ),
                ).with_priority(2))
            })
            .into_any_element()
    }

    /// Render GraphQL query editor tab
    fn render_graphql_query_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        div()
            .id("graphql-query-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Header with info
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(4.0))
                    .child(
                        div()
                            .size(px(20.0))
                            .bg(theme.colors.method_delete.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(ICON_PLAY, ICON_SM, theme.colors.method_delete))
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("GraphQL Query")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Write your query, mutation, or subscription")
                    )
            )
            // Query editor
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h(px(200.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    .child(self.graphql_query_editor.clone())
            )
            .into_any_element()
    }

    /// Render GraphQL variables editor tab
    fn render_graphql_variables_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        div()
            .id("graphql-variables-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Header with info
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(4.0))
                    .child(
                        div()
                            .size(px(20.0))
                            .bg(theme.colors.accent.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.0))
                            .text_color(theme.colors.accent)
                            .child("{ }")
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Variables")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("JSON object with query variables")
                    )
            )
            // Variables editor
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h(px(150.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    .child(self.graphql_variables_editor.clone())
            )
            .into_any_element()
    }

    /// Render WebSocket messages tab with connection controls and message history
    fn render_websocket_messages_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let ws_state = self.ws_state;
        let is_connected = ws_state == WsConnectionState::Connected;
        let is_connecting = ws_state == WsConnectionState::Connecting;
        let messages = self.ws_messages.clone();

        div()
            .id("websocket-messages-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Connection status & controls
            .child(
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
                                    .child("WebSocket")
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
                                    .child(match ws_state {
                                        WsConnectionState::Connected => "Connected",
                                        WsConnectionState::Connecting => "Connecting...",
                                        WsConnectionState::Disconnected => "Disconnected",
                                    })
                            )
                    )
                    // Connect/Disconnect button
                    .child(
                        div()
                            .id("ws-connect-btn")
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
                            .when(is_connecting, |el| {
                                el.cursor(gpui::CursorStyle::Arrow)
                                    .opacity(0.5)
                            })
                            .hover(|s| s.opacity(0.8))
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if is_connecting {
                                    return;
                                }
                                if is_connected {
                                    this.disconnect_websocket(cx);
                                } else {
                                    this.connect_websocket(cx);
                                }
                            }))
                            .child(if is_connected { "Disconnect" } else { "Connect" })
                    )
            )
            // Message history
            .child(
                div()
                    .id("ws-messages-container")
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
            // Message input
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_muted)
                            .child("MESSAGE")
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
                                    .child(self.ws_message_editor.clone())
                            )
                            .child(
                                div()
                                    .id("ws-send-btn")
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
                                            this.send_websocket_message(cx);
                                        }
                                    }))
                                    .child("Send")
                            )
                    )
            )
            .into_any_element()
    }

    /// Render gRPC Message tab
    fn render_grpc_message_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let has_service = self.grpc_service.is_some();
        let has_method = self.grpc_method.is_some();

        div()
            .id("grpc-message-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            // Service/Method selection row
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    // Service dropdown
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("Service")
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(px(32.0))
                                    .px(px(12.0))
                                    .bg(theme.colors.bg_secondary)
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.0))
                                    .text_color(if has_service { theme.colors.text_primary } else { theme.colors.text_muted })
                                    .child(
                                        self.grpc_service.clone()
                                            .unwrap_or_else(|| "Select service...".to_string())
                                    )
                            )
                    )
                    // Method dropdown
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("Method")
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(px(32.0))
                                    .px(px(12.0))
                                    .bg(theme.colors.bg_secondary)
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(if has_method { theme.colors.text_primary } else { theme.colors.text_muted })
                                    .child(
                                        self.grpc_method
                                            .as_ref()
                                            .map(|m| m.full_name.clone())
                                            .unwrap_or_else(|| "Select method...".to_string())
                                    )
                                    .when(self.grpc_method.as_ref().map(|m| m.streaming_type != GrpcStreamingType::Unary).unwrap_or(false), |el| {
                                        el.child(
                                            div()
                                                .id("grpc-streaming-badge")
                                                .px(px(6.0))
                                                .py(px(2.0))
                                                .bg(theme.colors.protocol_grpc.opacity(0.15))
                                                .text_size(px(9.0))
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(theme.colors.protocol_grpc)
                                                .child(
                                                    self.grpc_method.as_ref()
                                                        .map(|m| m.streaming_type.label())
                                                        .unwrap_or("")
                                                )
                                        )
                                    })
                            )
                    )
            )
            // Message editor
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child("Request Message (JSON)")
                    )
                    .child(
                        div()
                            .flex_1()
                            .border_1()
                            .border_color(theme.colors.border)
                            .overflow_hidden()
                            .child(self.grpc_message_editor.clone())
                    )
            )
            // Placeholder message when no proto loaded
            .when(self.grpc_proto_path.is_none(), |el| {
                el.child(
                    div()
                        .w_full()
                        .py(px(8.0))
                        .px(px(12.0))
                        .bg(theme.colors.accent.opacity(0.1))
                        .text_size(px(11.0))
                        .text_color(theme.colors.text_muted)
                        .child("Load a .proto file in the Proto tab to select services and methods")
                )
            })
            .into_any_element()
    }

    fn render_grpc_metadata_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let meta_len = self.grpc_metadata.len();
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();
        let enabled_count = self.grpc_metadata.iter().filter(|m| m.enabled && !m.key.is_empty()).count();

        let meta_data: Vec<_> = self.grpc_metadata.iter().enumerate().map(|(i, m)| {
            (i, m.enabled, m.key.clone(), m.value.clone())
        }).collect();

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .track_focus(&self.edit_focus);

        container = container.child(self.render_kv_table_header("KEY", "VALUE", enabled_count, cx));

        for (i, is_enabled, key, value) in meta_data {
            let can_remove = meta_len > 1;
            let is_editing_key = active_edit == Some(EditTarget::GrpcMetaKey(i));
            let is_editing_value = active_edit == Some(EditTarget::GrpcMetaValue(i));
            let is_row_editing = is_editing_key || is_editing_value;

            container = container.child(
                div()
                    .id(SharedString::from(format!("grpc-meta-row-{}", i)))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(4.0))
                    .px(px(2.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    .child(
                        self.render_kv_checkbox(format!("grpc-meta-cb-{}", i).into(), is_enabled, cx)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_grpc_meta(i, cx);
                            }))
                    )
                    .child(
                        self.render_kv_input(
                            format!("grpc-meta-key-{}", i),
                            EditTarget::GrpcMetaKey(i),
                            &key,
                            "metadata-key",
                            is_editing_key,
                            if is_editing_key { edit_selection.clone() } else { 0..0 },
                            px(self.kv_col_key_w),
                            cx,
                        )
                    )
                    .child(div().w(px(4.0)))
                    .child(
                        self.render_kv_input_flex(
                            format!("grpc-meta-val-{}", i),
                            EditTarget::GrpcMetaValue(i),
                            &value,
                            "value",
                            is_editing_value,
                            if is_editing_value { edit_selection.clone() } else { 0..0 },
                            cx,
                        )
                    )
                    .child(
                        self.render_kv_remove_btn(format!("grpc-meta-del-{}", i).into(), can_remove, cx)
                            .when(can_remove, |el| el.on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_grpc_meta(i, cx);
                            })))
                    )
            );
        }

        container = container.child(
            div().w_full().pt(px(8.0)).child(
                self.render_kv_add_btn("add-grpc-meta-btn", "+ Add Metadata", cx)
                    .on_click(cx.listener(|this, _, _, cx| { this.add_grpc_meta(cx); }))
            )
        );

        container.into_any_element()
    }

    /// Render gRPC Proto tab
    fn render_grpc_proto_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let has_proto = self.grpc_proto_path.is_some();
        let proto_path = self.grpc_proto_path.clone()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        div()
            .id("grpc-proto-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            // Load proto button row
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .id("load-proto-btn")
                            .px(px(16.0))
                            .py(px(8.0))
                            .bg(theme.colors.accent)
                            .cursor_pointer()
                            .hover(|s| s.opacity(0.9))
                            .text_size(px(12.0))
                            .text_color(theme.colors.bg_primary)
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.load_proto_file(cx);
                            }))
                            .child("Load .proto file")
                    )
                    .when(has_proto, |el| {
                        el.child(
                            div()
                                .flex_1()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted)
                                .overflow_x_hidden()
                                .child(proto_path)
                        )
                        .child(
                            div()
                                .id("clear-proto-btn")
                                .px(px(12.0))
                                .py(px(8.0))
                                .bg(theme.colors.bg_tertiary)
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.method_delete.opacity(0.1)))
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted)
                                .hover(|s| s.text_color(theme.colors.method_delete))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.grpc_proto_path = None;
                                    this.grpc_proto_content.clear();
                                    this.grpc_services.clear();
                                    this.grpc_service = None;
                                    this.grpc_methods.clear();
                                    this.grpc_method = None;
                                    cx.notify();
                                }))
                                .child("Clear")
                        )
                    })
            )
            // Proto content display
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child("Proto Definition")
                    )
                    .child(
                        div()
                            .id("proto-content-scroll")
                            .flex_1()
                            .w_full()
                            .border_1()
                            .border_color(theme.colors.border)
                            .bg(theme.colors.bg_secondary)
                            .p(px(12.0))
                            .overflow_scroll()
                            .when(has_proto, |el| {
                                el.child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(theme.colors.text_primary)
                                        .font_family("monospace")
                                        .child(self.grpc_proto_content.clone())
                                )
                            })
                            .when(!has_proto, |el| {
                                el.child(
                                    div()
                                        .w_full()
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_muted)
                                        .child("No proto file loaded")
                                )
                            })
                    )
            )
            .into_any_element()
    }

    /// Render import modal
    pub fn render_import_modal(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let _import_text = self.import_text.clone();
        let import_error = self.import_error.clone();

        // Full-screen overlay with centered modal
        div()
            .id("import-modal-overlay")
            .absolute()
            .inset_0()
            .bg(theme.colors.overlay)
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_import_modal(cx);
            }))
            .child(
                div()
                    .id("import-modal")
                    .w(px(600.0))
                    .bg(theme.colors.bg_primary)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }))
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }))
                    // Header
                    .child(
                        div()
                            .h(px(48.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("Import Request")
                            )
                            .child(
                                div()
                                    .id("close-import-modal")
                                    .size(px(28.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(14.0))
                                    .text_color(theme.colors.text_muted)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.close_import_modal(cx);
                                    }))
                                    .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted))
                            )
                    )
                    // Content
                    .child(
                        div()
                            .p(px(16.0))
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            // Instructions with browse button
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.text_secondary)
                                            .child("Paste cURL command or Postman collection:")
                                    )
                                    .child(
                                        div()
                                            .id("browse-import")
                                            .px(px(10.0))
                                            .py(px(5.0))
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_secondary)
                                            .cursor_pointer()
                                            .border_1()
                                            .border_color(theme.colors.border)
                                            .flex()
                                            .items_center()
                                            .gap(px(5.0))
                                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.browse_import_file(cx);
                                            }))
                                            .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_secondary))
                                            .child("Browse...")
                                    )
                            )
                            // Code editor (replaces plain textarea)
                            .child(
                                div()
                                    .id("import-editor-wrap")
                                    .w_full()
                                    .h(px(220.0))
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .overflow_hidden()
                                    .child(self.import_editor.clone())
                            )
                            // Error message
                            .when(import_error.is_some(), |el| {
                                el.child(
                                    div()
                                        .px(px(10.0))
                                        .py(px(8.0))
                                        .bg(theme.colors.status_client_error.opacity(0.1))
                                        .border_1()
                                        .border_color(theme.colors.status_client_error.opacity(0.3))
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.status_client_error)
                                        .child(import_error.unwrap_or_default())
                                )
                            })
                    )
                    // Footer
                    .child(
                        div()
                            .h(px(56.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.0))
                            .border_t_1()
                            .border_color(theme.colors.border)
                            // Clear button
                            .child(
                                div()
                                    .id("clear-import")
                                    .px(px(14.0))
                                    .py(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .cursor_pointer()
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_import_text(String::new(), cx);
                                    }))
                                    .child("Clear")
                            )
                            // Import button
                            .child(
                                div()
                                    .id("execute-import")
                                    .px(px(14.0))
                                    .py(px(8.0))
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.bg_primary)
                                    .bg(theme.colors.accent)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.accent_hover))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.execute_import(cx);
                                    }))
                                    .child("Import")
                            )
                    )
            )
            .into_any_element()
    }

    fn render_trpc_procedure_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let procedure = self.trpc_procedure.clone();
        let is_editing = self.active_edit == Some(EditTarget::TrpcProcedure);
        let edit_selection = self.edit_selection.clone();

        div()
            .id("trpc-procedure-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .p(px(16.0))
            .track_focus(&self.edit_focus)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("tRPC Procedure")
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.colors.text_muted)
                            .child("Enter the procedure name (e.g., \"query.getUser\", \"mutation.createPost\")")
                    )
                    .child(
                        self.render_kv_input_flex(
                            "trpc-procedure-input".to_string(),
                            EditTarget::TrpcProcedure,
                            &procedure,
                            "query.getUser",
                            is_editing,
                            if is_editing { edit_selection } else { 0..0 },
                            cx,
                        )
                    )
            )
            .into_any_element()
    }

    fn render_trpc_parameters_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        div()
            .id("trpc-parameters-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(4.0))
                    .child(
                        div()
                            .size(px(20.0))
                            .bg(theme.colors.method_post.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.0))
                            .text_color(theme.colors.method_post)
                            .child("{ }")
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Parameters (JSON)")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Enter the parameters for your tRPC procedure")
                    )
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    .child(self.trpc_params_editor.clone())
            )
            .into_any_element()
    }

    /// Render Socket.IO events tab: namespace + event name inputs, payload editor, send, history.
    fn render_socketio_events_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
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
            // ── Status bar ────────────────────────────────────────────────
            .child(
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
            )
            // ── Namespace + event name row ─────────────────────────────────
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
            // ── Event history ─────────────────────────────────────────────
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
                            }))
                    })
            )
            // ── Emit composer ─────────────────────────────────────────────
            .child(
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
                            // Ack toggle
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
            )
            .into_any_element()
    }
}

