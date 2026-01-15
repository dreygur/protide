//! Rendering methods for RequestPanel

use std::ops::Range;

use gpui::{
    div, prelude::*, px, Context, IntoElement, KeyDownEvent, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, SharedString, Styled, Window,
};

use crate::theme;
use crate::codegen::Language as CodegenLanguage;
use crate::ui::components::render_text_view_with_max;
use super::super::request_types::{ApiKeyLocation, AuthType, BodyType, EditTarget, FormFieldType, HttpMethod, RequestMode, WsConnectionState, WsMessageDirection};
use super::{render_text_view, RequestPanel};

impl RequestPanel {
    pub(super) fn render_url_bar(&mut self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let method = self.method;
        let method_color = theme.method_color(method.as_str());
        let is_url_focused = self.url_focus.is_focused(window);

        // Detect protocol
        let is_https = self.url.starts_with("https://");
        let is_http = self.url.starts_with("http://");
        let protocol_color = if is_https {
            theme.colors.status_success
        } else if is_http {
            theme.colors.method_patch // orange/yellow for http
        } else {
            theme.colors.text_muted
        };

        // Calculate URL input text position from window left edge
        let base_offset = 360.0;
        let protocol_offset = if is_https || is_http { 70.0 } else { 0.0 };
        self.url_input_left = base_offset + protocol_offset;

        let is_graphql = self.request_mode == RequestMode::GraphQL;

        div()
            .w_full()
            .h(px(56.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(16.0))
            .bg(theme.colors.bg_secondary)
            .border_b_1()
            .border_color(theme.colors.border)
            // Mode toggle (HTTP/GraphQL/WebSocket)
            .child(
                div()
                    .id("mode-toggle")
                    .flex()
                    .items_center()
                    .h(px(32.0))
                    .rounded(px(6.0))
                    .bg(theme.colors.bg_tertiary)
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    // HTTP button
                    .child(
                        div()
                            .id("mode-http")
                            .px(px(10.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .cursor_pointer()
                            .when(self.request_mode == RequestMode::Http, |el| {
                                el.bg(theme.colors.accent.opacity(0.15))
                                    .text_color(theme.colors.accent)
                            })
                            .when(self.request_mode != RequestMode::Http, |el| {
                                el.text_color(theme.colors.text_secondary)
                                    .hover(|s| s.bg(theme.colors.bg_secondary))
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_request_mode(RequestMode::Http, cx);
                            }))
                            .child("HTTP")
                    )
                    // GraphQL button
                    .child(
                        div()
                            .id("mode-graphql")
                            .px(px(10.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .cursor_pointer()
                            .when(is_graphql, |el| {
                                el.bg(theme.colors.method_delete.opacity(0.15))
                                    .text_color(theme.colors.method_delete)
                            })
                            .when(!is_graphql, |el| {
                                el.text_color(theme.colors.text_secondary)
                                    .hover(|s| s.bg(theme.colors.bg_secondary))
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_request_mode(RequestMode::GraphQL, cx);
                            }))
                            .child("GraphQL")
                    )
                    // WebSocket button
                    .child(
                        div()
                            .id("mode-ws")
                            .px(px(10.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .cursor_pointer()
                            .when(self.request_mode == RequestMode::WebSocket, |el| {
                                el.bg(theme.colors.method_get.opacity(0.15))
                                    .text_color(theme.colors.method_get)
                            })
                            .when(self.request_mode != RequestMode::WebSocket, |el| {
                                el.text_color(theme.colors.text_secondary)
                                    .hover(|s| s.bg(theme.colors.bg_secondary))
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_request_mode(RequestMode::WebSocket, cx);
                            }))
                            .child("WS")
                    )
            )
            // Method selector button (dropdown rendered separately as overlay) - only show for HTTP mode
            .when(self.request_mode == RequestMode::Http, |el| {
                el.child(
                    div()
                        .id("method-selector")
                        .min_w(px(72.0))
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(6.0))
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
                        .child(method.as_str())
                        .child(
                            div()
                                .text_size(px(8.0))
                                .text_color(method_color.opacity(0.7))
                                .child("▼")
                        )
                )
            })
            // Protocol indicator
            .when(is_https || is_http, |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .px(px(8.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
                        .bg(protocol_color.opacity(0.1))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .when(is_https, |el| el.child("🔒"))
                                .when(is_http, |el| el.child("⚠"))
                        )
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(protocol_color)
                                .when(is_https, |el| el.child("HTTPS"))
                                .when(is_http, |el| el.child("HTTP"))
                        )
                )
            })
            // URL input with selection support
            .child(
                div()
                    .id("url-input")
                    .flex_1()
                    .min_w(px(0.0))
                    .h(px(36.0))
                    .px(px(14.0))
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .rounded(px(6.0))
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
                    .child(self.render_url_text(is_url_focused, cx)),
            )
            // Send button with shortcut hint
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child({
                        let is_loading = self.loading;
                        div()
                            .id("send-button")
                            .h(px(36.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(6.0))
                            .rounded(px(6.0))
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(gpui::white())
                            .when(is_loading, |el| {
                                el.bg(theme.colors.accent.opacity(0.7))
                                    .cursor_default()
                                    // Spinner
                                    .child(
                                        div()
                                            .size(px(14.0))
                                            .rounded_full()
                                            .border_2()
                                            .border_color(gpui::white().opacity(0.3))
                                            .border_t_2()
                                            .border_color(gpui::white())
                                    )
                                    .child("Sending...")
                            })
                            .when(!is_loading, |el| {
                                el.bg(theme.colors.accent)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.colors.accent_hover))
                                    .active(|style| style.opacity(0.9))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.send_request(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .child("▶")
                                    )
                                    .child("Send")
                            })
                    })
                    // Keyboard shortcut hint
                    .child(
                        div()
                            .px(px(6.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.colors.bg_tertiary)
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("⌘↵")
                    )
                    // Save button
                    .child(
                        div()
                            .id("save-button")
                            .h(px(32.0))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(4.0))
                            .rounded(px(6.0))
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_secondary)
                            .cursor_pointer()
                            .border_1()
                            .border_color(theme.colors.border)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.save_request(cx);
                            }))
                            .child("💾")
                            .child("Save")
                    )
                    // Code generation button
                    .child(
                        div()
                            .id("code-button")
                            .h(px(32.0))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(4.0))
                            .rounded(px(6.0))
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_secondary)
                            .cursor_pointer()
                            .border_1()
                            .border_color(theme.colors.border)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_codegen_dropdown(cx);
                            }))
                            .child("</> Code")
                    )
                    // Import button
                    .child(
                        div()
                            .id("import-button")
                            .h(px(32.0))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(4.0))
                            .rounded(px(6.0))
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_secondary)
                            .cursor_pointer()
                            .border_1()
                            .border_color(theme.colors.border)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_import_modal(cx);
                            }))
                            .child("↓ Import")
                    )
            )
    }

    pub(super) fn render_url_text(&self, is_focused: bool, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        render_text_view(
            &self.url,
            &self.url_selection,
            is_focused,
            13.0,
            theme.colors.text_primary,
            Some("Enter request URL..."),
            theme.colors.text_muted,
        )
    }

    pub(super) fn render_method_dropdown_overlay(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        // Positioned at top-left of panel, below the URL bar
        div()
            .id("method-dropdown-overlay")
            .absolute()
            .top(px(46.0))  // Below URL bar
            .left(px(16.0)) // Same padding as URL bar
            .min_w(px(100.0))
            .py(px(6.0))
            .rounded(px(8.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.skip_blur = true;
                cx.stop_propagation();
            }))
            .children(HttpMethod::all().iter().map(|&m| {
                let method_color = theme.method_color(m.as_str());
                let is_selected = m == self.method;

                div()
                    .id(SharedString::from(format!("method-{}", m.as_str())))
                    .mx(px(4.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(4.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .cursor_pointer()
                    .when(is_selected, |el| {
                        el.bg(method_color.opacity(0.1))
                            .child(
                                div()
                                    .size(px(6.0))
                                    .rounded_full()
                                    .bg(method_color)
                            )
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
                            .child(m.as_str())
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_method(m, cx);
                    }))
            }))
    }

    pub(super) fn render_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let tabs: &[(&str, &str)] = match self.request_mode {
            RequestMode::GraphQL => &[("Query", "◇"), ("Variables", "{ }"), ("Headers", "H"), ("Auth", "🔒")],
            RequestMode::WebSocket => &[("Messages", "⚡"), ("Headers", "H")],
            RequestMode::Http => &[("Params", "?"), ("Headers", "H"), ("Body", "{ }"), ("Auth", "🔒"), ("Scripts", "ƒ")],
        };
        let active_tab = self.active_tab;

        // Get counts for badges
        let param_count = self.params.iter().filter(|p| p.enabled && !p.key.is_empty()).count();
        let header_count = self.headers.iter().filter(|h| h.enabled && !h.key.is_empty()).count();

        div()
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .px(px(12.0))
            .gap(px(2.0))
            .border_b_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_secondary)
            .children(tabs.iter().enumerate().map(|(i, (tab, icon))| {
                let is_active = i == active_tab;
                let count = match self.request_mode {
                    RequestMode::GraphQL => match i {
                        2 => header_count, // Headers is at index 2 for GraphQL
                        _ => 0,
                    },
                    RequestMode::WebSocket => match i {
                        1 => header_count, // Headers is at index 1 for WebSocket
                        _ => 0,
                    },
                    RequestMode::Http => match i {
                        0 => param_count,
                        1 => header_count,
                        _ => 0,
                    },
                };

                div()
                    .id(SharedString::from(format!("tab-{}", i)))
                    .px(px(14.0))
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded_t(px(6.0))
                    .cursor_pointer()
                    .when(is_active, |el| {
                        el.bg(theme.colors.bg_tertiary)
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.accent)
                                    .child(*icon)
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_primary)
                                    .child(*tab)
                            )
                            .when(count > 0, |el| {
                                el.child(
                                    div()
                                        .px(px(5.0))
                                        .py(px(1.0))
                                        .rounded(px(8.0))
                                        .bg(theme.colors.accent.opacity(0.15))
                                        .text_size(px(10.0))
                                        .text_color(theme.colors.accent)
                                        .child(format!("{}", count))
                                )
                            })
                    })
                    .when(!is_active, |el| {
                        el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.5)))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(*icon)
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child(*tab)
                            )
                            .when(count > 0, |el| {
                                el.child(
                                    div()
                                        .px(px(5.0))
                                        .py(px(1.0))
                                        .rounded(px(8.0))
                                        .bg(theme.colors.bg_tertiary)
                                        .text_size(px(10.0))
                                        .text_color(theme.colors.text_muted)
                                        .child(format!("{}", count))
                                )
                            })
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_tab(i, cx);
                    }))
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
        }
    }

    fn render_params_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let params_len = self.params.len();
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
            .track_focus(&self.edit_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_edit_key(event, cx);
            }));

        // Table header
        container = container.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.0))
                .pb(px(8.0))
                .border_b_1()
                .border_color(theme.colors.border.opacity(0.5))
                .mb(px(4.0))
                // Checkbox spacer
                .child(div().size(px(16.0)))
                // Key column header
                .child(
                    div()
                        .w(px(150.0))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text_muted)
                        .child("KEY")
                )
                // Value column header
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.colors.text_muted)
                                .child("VALUE")
                        )
                        .child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(8.0))
                                .bg(theme.colors.accent.opacity(0.12))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.accent)
                                .child(format!("{} active", enabled_count))
                        )
                )
                // Action spacer
                .child(div().size(px(24.0)))
        );

        // Params list
        for (i, is_enabled, key, value) in params_data {
            let can_remove = params_len > 1;
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
                    .rounded(px(4.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    // Checkbox
                    .child(
                        div()
                            .id(SharedString::from(format!("param-checkbox-{}", i)))
                            .size(px(16.0))
                            .rounded(px(4.0))
                            .border_1()
                            .cursor_pointer()
                            .when(is_enabled, |el| {
                                el.bg(theme.colors.accent)
                                    .border_color(theme.colors.accent)
                            })
                            .when(!is_enabled, |el| {
                                el.border_color(theme.colors.border)
                                    .hover(|s| s.border_color(theme.colors.text_muted))
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(is_enabled, |el| {
                                el.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(gpui::white())
                                        .child("✓")
                                )
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_param(i, cx);
                            }))
                    )
                    // Key input
                    .child(
                        self.render_kv_input(
                            format!("param-key-{}", i),
                            EditTarget::ParamKey(i),
                            &key,
                            "Key",
                            is_editing_key,
                            if is_editing_key { edit_selection.clone() } else { 0..0 },
                            px(150.0),
                            cx,
                        )
                    )
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
                    // Remove button
                    .child(
                        div()
                            .id(SharedString::from(format!("param-remove-{}", i)))
                            .size(px(24.0))
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(14.0))
                            .when(can_remove, |el| {
                                el.text_color(theme.colors.text_muted)
                                    .hover(|s| s.bg(theme.colors.status_client_error.opacity(0.1)).text_color(theme.colors.status_client_error))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.remove_param(i, cx);
                                    }))
                            })
                            .when(!can_remove, |el| el.text_color(theme.colors.border))
                            .child("×")
                    )
            );
        }

        // Add param button
        container = container.child(
            div()
                .id("add-param-btn")
                .mt(px(8.0))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .cursor_pointer()
                .text_size(px(12.0))
                .text_color(theme.colors.accent)
                .border_1()
                .border_color(theme.colors.accent.opacity(0.3))
                .hover(|s| s.bg(theme.colors.accent.opacity(0.08)).border_color(theme.colors.accent.opacity(0.5)))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.add_param(cx);
                }))
                .child(
                    div()
                        .text_size(px(14.0))
                        .child("+")
                )
                .child("Add Parameter")
        );

        container.into_any_element()
    }

    fn render_headers_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let headers_len = self.headers.len();
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
            .track_focus(&self.edit_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_edit_key(event, cx);
            }));

        // Table header
        container = container.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.0))
                .pb(px(8.0))
                .border_b_1()
                .border_color(theme.colors.border.opacity(0.5))
                .mb(px(4.0))
                // Checkbox spacer
                .child(div().size(px(16.0)))
                // Key column header
                .child(
                    div()
                        .w(px(150.0))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text_muted)
                        .child("HEADER")
                )
                // Value column header
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.colors.text_muted)
                                .child("VALUE")
                        )
                        .child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(8.0))
                                .bg(theme.colors.accent.opacity(0.12))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.accent)
                                .child(format!("{} active", enabled_count))
                        )
                )
                // Action spacer
                .child(div().size(px(24.0)))
        );

        // Headers list
        for (i, is_enabled, key, value) in headers_data {
            let can_remove = headers_len > 1;
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
                    .rounded(px(4.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    // Checkbox
                    .child(
                        div()
                            .id(SharedString::from(format!("header-checkbox-{}", i)))
                            .size(px(16.0))
                            .rounded(px(4.0))
                            .border_1()
                            .cursor_pointer()
                            .when(is_enabled, |el| {
                                el.bg(theme.colors.accent)
                                    .border_color(theme.colors.accent)
                            })
                            .when(!is_enabled, |el| {
                                el.border_color(theme.colors.border)
                                    .hover(|s| s.border_color(theme.colors.text_muted))
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(is_enabled, |el| {
                                el.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(gpui::white())
                                        .child("✓")
                                )
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_header(i, cx);
                            }))
                    )
                    // Key input
                    .child(
                        self.render_kv_input(
                            format!("header-key-{}", i),
                            EditTarget::HeaderKey(i),
                            &key,
                            "Header name",
                            is_editing_key,
                            if is_editing_key { edit_selection.clone() } else { 0..0 },
                            px(150.0),
                            cx,
                        )
                    )
                    // Value input
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
                    // Remove button
                    .child(
                        div()
                            .id(SharedString::from(format!("header-remove-{}", i)))
                            .size(px(24.0))
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(14.0))
                            .when(can_remove, |el| {
                                el.text_color(theme.colors.text_muted)
                                    .hover(|s| s.bg(theme.colors.status_client_error.opacity(0.1)).text_color(theme.colors.status_client_error))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.remove_header(i, cx);
                                    }))
                            })
                            .when(!can_remove, |el| el.text_color(theme.colors.border))
                            .child("×")
                    )
            );
        }

        // Add header button
        container = container.child(
            div()
                .id("add-header-btn")
                .mt(px(8.0))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .cursor_pointer()
                .text_size(px(12.0))
                .text_color(theme.colors.accent)
                .border_1()
                .border_color(theme.colors.accent.opacity(0.3))
                .hover(|s| s.bg(theme.colors.accent.opacity(0.08)).border_color(theme.colors.accent.opacity(0.5)))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.add_header(cx);
                }))
                .child(
                    div()
                        .text_size(px(14.0))
                        .child("+")
                )
                .child("Add Header")
        );

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
            .track_focus(&self.edit_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_edit_key(event, cx);
            }));

        // Toolbar row with body type buttons
        container = container.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .mb(px(8.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .p(px(3.0))
                        .rounded(px(6.0))
                        .bg(theme.colors.bg_tertiary)
                        .child(
                            div()
                                .id("body-type-json-form")
                                .px(px(10.0))
                                .py(px(5.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .text_color(theme.colors.text_muted)
                                .hover(|s| s.text_color(theme.colors.text_secondary))
                                .text_size(px(11.0))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_body_type(BodyType::Json, cx);
                                }))
                                .child("JSON")
                        )
                        .child(
                            div()
                                .id("body-type-raw-form")
                                .px(px(10.0))
                                .py(px(5.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .text_color(theme.colors.text_muted)
                                .hover(|s| s.text_color(theme.colors.text_secondary))
                                .text_size(px(11.0))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_body_type(BodyType::Raw, cx);
                                }))
                                .child("Raw")
                        )
                        .child(
                            div()
                                .id("body-type-form-form")
                                .px(px(10.0))
                                .py(px(5.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .bg(theme.colors.bg_primary)
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.text_primary)
                                .text_size(px(11.0))
                                .child("Form")
                        )
                )
                .child(
                    div()
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(8.0))
                        .bg(theme.colors.accent.opacity(0.12))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.colors.accent)
                        .child(format!("{} fields", enabled_count))
                )
        );

        // Table header
        container = container.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.0))
                .pb(px(8.0))
                .border_b_1()
                .border_color(theme.colors.border.opacity(0.5))
                .mb(px(4.0))
                .child(div().size(px(16.0)))
                .child(
                    div()
                        .w(px(130.0))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text_muted)
                        .child("KEY")
                )
                .child(
                    div()
                        .w(px(50.0))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text_muted)
                        .child("TYPE")
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text_muted)
                        .child("VALUE")
                )
                .child(div().size(px(24.0)))
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
                    .rounded(px(4.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    // Checkbox
                    .child(
                        div()
                            .id(SharedString::from(format!("form-checkbox-{}", i)))
                            .size(px(16.0))
                            .rounded(px(4.0))
                            .border_1()
                            .cursor_pointer()
                            .when(is_enabled, |el| {
                                el.bg(theme.colors.accent)
                                    .border_color(theme.colors.accent)
                                    .child(
                                        div()
                                            .size_full()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_color(gpui::white())
                                            .text_size(px(10.0))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .child("✓")
                                    )
                            })
                            .when(!is_enabled, |el| {
                                el.border_color(theme.colors.border)
                                    .hover(|s| s.border_color(theme.colors.text_muted))
                            })
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
                            .rounded(px(4.0))
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
                                        .rounded(px(4.0))
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
                    // Remove button
                    .child(
                        div()
                            .id(SharedString::from(format!("form-remove-{}", i)))
                            .size(px(24.0))
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .when(can_remove, |el| {
                                el.text_color(theme.colors.text_muted)
                                    .hover(|s| s.bg(theme.colors.status_client_error.opacity(0.1)).text_color(theme.colors.status_client_error))
                                    .on_click({
                                        let idx = i;
                                        cx.listener(move |this, _, _, cx| {
                                            this.remove_form_field(idx, cx);
                                        })
                                    })
                            })
                            .when(!can_remove, |el| {
                                el.text_color(theme.colors.text_muted.opacity(0.3))
                            })
                            .text_size(px(14.0))
                            .child("×")
                    )
            );
        }

        // Add field button
        container = container.child(
            div()
                .w_full()
                .pt(px(8.0))
                .child(
                    div()
                        .id("add-form-field-btn")
                        .w_full()
                        .py(px(8.0))
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(theme.colors.border.opacity(0.5))
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(6.0))
                        .cursor_pointer()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .hover(|s| {
                            s.bg(theme.colors.bg_tertiary)
                                .border_color(theme.colors.border)
                                .text_color(theme.colors.text_secondary)
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.add_form_field(cx);
                        }))
                        .child("+")
                        .child("Add Field")
                )
        );

        container.into_any_element()
    }

    fn render_body_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        // For Form type, render KV editor instead of text editor
        if self.body_type == BodyType::Form {
            return self.render_form_body(cx);
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
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    // Left: Body type selector
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .p(px(3.0))
                            .rounded(px(6.0))
                            .bg(theme.colors.bg_tertiary)
                            .child(
                                div()
                                    .id("body-type-json")
                                    .px(px(10.0))
                                    .py(px(5.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .when(self.body_type == BodyType::Json, |el| {
                                        el.bg(theme.colors.bg_primary)
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_primary)
                                    })
                                    .when(self.body_type != BodyType::Json, |el| {
                                        el.text_color(theme.colors.text_muted)
                                            .hover(|s| s.text_color(theme.colors.text_secondary))
                                    })
                                    .text_size(px(11.0))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_body_type(BodyType::Json, cx);
                                    }))
                                    .child("JSON")
                            )
                            .child(
                                div()
                                    .id("body-type-raw")
                                    .px(px(10.0))
                                    .py(px(5.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .when(self.body_type == BodyType::Raw, |el| {
                                        el.bg(theme.colors.bg_primary)
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_primary)
                                    })
                                    .when(self.body_type != BodyType::Raw, |el| {
                                        el.text_color(theme.colors.text_muted)
                                            .hover(|s| s.text_color(theme.colors.text_secondary))
                                    })
                                    .text_size(px(11.0))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_body_type(BodyType::Raw, cx);
                                    }))
                                    .child("Raw")
                            )
                            .child(
                                div()
                                    .id("body-type-form")
                                    .px(px(10.0))
                                    .py(px(5.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .when(self.body_type == BodyType::Form, |el| {
                                        el.bg(theme.colors.bg_primary)
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_primary)
                                    })
                                    .when(self.body_type != BodyType::Form, |el| {
                                        el.text_color(theme.colors.text_muted)
                                            .hover(|s| s.text_color(theme.colors.text_secondary))
                                    })
                                    .text_size(px(11.0))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_body_type(BodyType::Form, cx);
                                    }))
                                    .child("Form")
                            )
                    )
                    // Right: Info label
                    .child(
                        div()
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
            .rounded(px(4.0))
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
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
            // ~18 chars for 150px width (accounting for padding)
            .child(self.render_kv_text(&text, placeholder, is_editing, selection, Some(18), cx))
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
            .rounded(px(4.0))
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
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
            // ~35 chars for flex width (reasonable default)
            .child(self.render_kv_text(&text, placeholder, is_editing, selection, Some(35), cx))
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
        )
    }

    fn render_auth_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let auth_type = self.auth_type;
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();

        let auth_types = [
            (AuthType::None, "None", "○"),
            (AuthType::Bearer, "Bearer", "🎫"),
            (AuthType::Basic, "Basic", "👤"),
            (AuthType::ApiKey, "API Key", "🔑"),
        ];

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .track_focus(&self.edit_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_edit_key(event, cx);
            }))
            // Auth type selector with pill style
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .p(px(3.0))
                    .rounded(px(8.0))
                    .bg(theme.colors.bg_tertiary)
                    .children(auth_types.iter().map(|(at, label, icon)| {
                        let is_selected = *at == auth_type;
                        let at = *at;
                        div()
                            .id(SharedString::from(format!("auth-type-{:?}", at)))
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .text_size(px(12.0))
                            .when(is_selected, |el| {
                                el.bg(theme.colors.bg_primary)
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_primary)
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .child(*icon)
                                    )
                            })
                            .when(!is_selected, |el| {
                                el.text_color(theme.colors.text_muted)
                                    .hover(|s| s.text_color(theme.colors.text_secondary).bg(theme.colors.bg_secondary.opacity(0.5)))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_auth_type(at, cx);
                            }))
                            .child(*label)
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
                    .rounded(px(8.0))
                    .bg(theme.colors.bg_tertiary.opacity(0.5))
                    .border_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .size(px(32.0))
                            .rounded(px(6.0))
                            .bg(theme.colors.text_muted.opacity(0.1))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(16.0))
                            .text_color(theme.colors.text_muted)
                            .child("○")
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
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
                    .rounded(px(8.0))
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
                                    .rounded(px(6.0))
                                    .bg(theme.colors.accent.opacity(0.1))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(14.0))
                                    .child("🎫")
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
                    .rounded(px(8.0))
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
                                    .rounded(px(6.0))
                                    .bg(theme.colors.accent.opacity(0.1))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(14.0))
                                    .child("👤")
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
                    .rounded(px(8.0))
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
                                    .rounded(px(6.0))
                                    .bg(theme.colors.accent.opacity(0.1))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(14.0))
                                    .child("🔑")
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
                                    .rounded(px(6.0))
                                    .bg(theme.colors.bg_primary)
                                    .child(
                                        div()
                                            .id("api-key-header")
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .px(px(10.0))
                                            .py(px(5.0))
                                            .rounded(px(4.0))
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
                                            .rounded(px(4.0))
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
            .rounded(px(4.0))
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
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
            // ~50 chars for 400px max width
            .child(self.render_kv_text(&display_text, placeholder, is_editing, selection, Some(50), cx))
    }

    fn render_scripts_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        div()
            .id("scripts-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .overflow_scroll()
            // Pre-request Script section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .size(px(20.0))
                                    .rounded(px(4.0))
                                    .bg(theme.colors.method_post.opacity(0.15))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.method_post)
                                    .child("▶")
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
                    .child(
                        div()
                            .h(px(150.0))
                            .w_full()
                            .border_1()
                            .border_color(theme.colors.border)
                            .rounded(px(6.0))
                            .overflow_hidden()
                            .child(self.pre_script_editor.clone())
                    )
            )
            // Post-response Script section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .size(px(20.0))
                                    .rounded(px(4.0))
                                    .bg(theme.colors.status_success.opacity(0.15))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.status_success)
                                    .child("◀")
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
                    .child(
                        div()
                            .h(px(150.0))
                            .w_full()
                            .border_1()
                            .border_color(theme.colors.border)
                            .rounded(px(6.0))
                            .overflow_hidden()
                            .child(self.post_script_editor.clone())
                    )
            )
            // Tests section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .size(px(20.0))
                                    .rounded(px(4.0))
                                    .bg(theme.colors.accent.opacity(0.15))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.accent)
                                    .child("✓")
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
                    .child(
                        div()
                            .h(px(150.0))
                            .w_full()
                            .border_1()
                            .border_color(theme.colors.border)
                            .rounded(px(6.0))
                            .overflow_hidden()
                            .child(self.tests_editor.clone())
                    )
            )
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
                            .rounded(px(4.0))
                            .bg(theme.colors.method_delete.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.0))
                            .text_color(theme.colors.method_delete)
                            .child("◇")
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
                    .rounded(px(6.0))
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
                            .rounded(px(4.0))
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
                    .rounded(px(6.0))
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
                                    .rounded(px(4.0))
                                    .bg(if is_connected {
                                        theme.colors.method_get.opacity(0.15)
                                    } else {
                                        theme.colors.text_muted.opacity(0.15)
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(10.0))
                                    .text_color(if is_connected {
                                        theme.colors.method_get
                                    } else {
                                        theme.colors.text_muted
                                    })
                                    .child("⚡")
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
                                    .rounded(px(4.0))
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
                            .rounded(px(6.0))
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
                    .rounded(px(6.0))
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
                                let is_sent = msg.direction == WsMessageDirection::Sent;
                                let time_str = msg.timestamp.format("%H:%M:%S").to_string();
                                div()
                                    .id(SharedString::from(format!("ws-msg-{}", i)))
                                    .w_full()
                                    .p(px(8.0))
                                    .rounded(px(4.0))
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
                                                    .text_size(px(10.0))
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(if is_sent {
                                                        theme.colors.accent
                                                    } else {
                                                        theme.colors.method_get
                                                    })
                                                    .child(if is_sent { "→ Sent" } else { "← Received" })
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
                                            .font_family("Ubuntu Mono")
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
                                    .rounded(px(6.0))
                                    .overflow_hidden()
                                    .child(self.ws_message_editor.clone())
                            )
                            .child(
                                div()
                                    .id("ws-send-btn")
                                    .h(px(80.0))
                                    .w(px(70.0))
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(is_connected, |el| {
                                        el.bg(theme.colors.accent)
                                            .hover(|s| s.opacity(0.9))
                                            .text_color(gpui::rgb(0xFFFFFF))
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

    /// Render code generation language dropdown overlay
    pub(super) fn render_codegen_dropdown_overlay(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        let languages = [
            (CodegenLanguage::Curl, "cURL", "curl command"),
            (CodegenLanguage::Python, "Python", "requests library"),
            (CodegenLanguage::JavaScript, "JavaScript", "fetch API"),
            (CodegenLanguage::Go, "Go", "net/http"),
            (CodegenLanguage::Rust, "Rust", "reqwest"),
        ];

        // Position near the Code button (right side of URL bar)
        div()
            .id("codegen-dropdown-overlay")
            .absolute()
            .top(px(46.0))
            .right(px(16.0))
            .min_w(px(180.0))
            .py(px(6.0))
            .rounded(px(8.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.skip_blur = true;
                cx.stop_propagation();
            }))
            .children(languages.iter().map(|&(lang, name, desc)| {
                let is_selected = lang == self.codegen_language;

                div()
                    .id(SharedString::from(format!("codegen-{}", name)))
                    .mx(px(4.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(4.0))
                    .flex()
                    .flex_col()
                    .cursor_pointer()
                    .when(is_selected, |el| {
                        el.bg(theme.colors.accent.opacity(0.1))
                    })
                    .when(!is_selected, |el| {
                        el.hover(|s| s.bg(theme.colors.bg_tertiary))
                    })
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child(name)
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child(desc)
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.generate_code(lang, cx);
                    }))
            }))
    }

    /// Render code generation modal with generated code
    pub(super) fn render_codegen_modal(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let code = self.codegen_content.clone().unwrap_or_default();
        let lang_name = match self.codegen_language {
            CodegenLanguage::Curl => "cURL",
            CodegenLanguage::Python => "Python",
            CodegenLanguage::JavaScript => "JavaScript",
            CodegenLanguage::Go => "Go",
            CodegenLanguage::Rust => "Rust",
        };

        // Full-screen overlay with centered modal
        div()
            .id("codegen-modal-overlay")
            .absolute()
            .inset_0()
            .bg(gpui::black().opacity(0.5))
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_codegen_modal(cx);
            }))
            .child(
                div()
                    .id("codegen-modal")
                    .w(px(700.0))
                    .max_h(px(500.0))
                    .bg(theme.colors.bg_primary)
                    .border_1()
                    .border_color(theme.colors.border)
                    .rounded(px(12.0))
                    .shadow_lg()
                    .flex()
                    .flex_col()
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
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(theme.colors.text_primary)
                                            .child(format!("Generated {} Code", lang_name))
                                    )
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    // Copy button
                                    .child(
                                        div()
                                            .id("copy-codegen")
                                            .px(px(10.0))
                                            .py(px(6.0))
                                            .rounded(px(4.0))
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_secondary)
                                            .cursor_pointer()
                                            .bg(theme.colors.bg_secondary)
                                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.copy_generated_code(cx);
                                            }))
                                            .child("📋 Copy")
                                    )
                                    // Close button
                                    .child(
                                        div()
                                            .id("close-codegen-modal")
                                            .size(px(28.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(4.0))
                                            .text_size(px(14.0))
                                            .text_color(theme.colors.text_muted)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.close_codegen_modal(cx);
                                            }))
                                            .child("✕")
                                    )
                            )
                    )
                    // Code content
                    .child(
                        div()
                            .id("codegen-content")
                            .flex_1()
                            .p(px(16.0))
                            .overflow_scroll()
                            .child(
                                div()
                                    .w_full()
                                    .p(px(12.0))
                                    .bg(theme.colors.bg_secondary)
                                    .rounded(px(6.0))
                                    .font_family("Ubuntu Mono")
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_primary)
                                    .child(code)
                            )
                    )
            )
    }

    /// Render import modal
    pub(super) fn render_import_modal(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let import_text = self.import_text.clone();
        let import_error = self.import_error.clone();

        // Full-screen overlay with centered modal
        div()
            .id("import-modal-overlay")
            .absolute()
            .inset_0()
            .bg(gpui::black().opacity(0.5))
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
                    .rounded(px(12.0))
                    .shadow_lg()
                    .flex()
                    .flex_col()
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
                                    .rounded(px(4.0))
                                    .text_size(px(14.0))
                                    .text_color(theme.colors.text_muted)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.close_import_modal(cx);
                                    }))
                                    .child("✕")
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
                                            .rounded(px(4.0))
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_secondary)
                                            .cursor_pointer()
                                            .border_1()
                                            .border_color(theme.colors.border)
                                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.browse_import_file(cx);
                                            }))
                                            .child("📁 Browse...")
                                    )
                            )
                            // Text area
                            .child(
                                div()
                                    .id("import-textarea")
                                    .w_full()
                                    .h(px(200.0))
                                    .p(px(12.0))
                                    .bg(theme.colors.bg_secondary)
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .rounded(px(6.0))
                                    .overflow_scroll()
                                    .font_family("Ubuntu Mono")
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_primary)
                                    .child(
                                        div()
                                            .id("import-input")
                                            .size_full()
                                            .cursor_text()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                // Read from clipboard on click if empty
                                                if this.import_text.is_empty() {
                                                    if let Some(clipboard) = cx.read_from_clipboard() {
                                                        if let Some(text) = clipboard.text() {
                                                            this.set_import_text(text.to_string(), cx);
                                                        }
                                                    }
                                                }
                                            }))
                                            .when(import_text.is_empty(), |el| {
                                                el.child(
                                                    div()
                                                        .text_color(theme.colors.text_muted)
                                                        .child("Click to paste from clipboard...")
                                                )
                                            })
                                            .when(!import_text.is_empty(), |el| {
                                                el.child(import_text.clone())
                                            })
                                    )
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
                                        .rounded(px(4.0))
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
                                    .rounded(px(6.0))
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
                                    .rounded(px(6.0))
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(gpui::white())
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
    }
}

