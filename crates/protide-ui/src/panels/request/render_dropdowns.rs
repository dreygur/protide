//! Dropdown overlay rendering for RequestPanel

use std::ops::Range;

use gpui::{
    div, prelude::*, px, Context, IntoElement, KeyDownEvent,
    ParentElement, SharedString, Styled, Window,
};

use crate::theme;
use crate::components::icons::{
    icon, ICON_SM,
    ICON_CHEVRON_DOWN,
};
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::{HttpMethod, RequestMode};
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_method_dropdown_overlay(&mut self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .id("method-dropdown-overlay")
            .absolute()
            .top(px(64.0))
            .left(px(142.0))
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

        div()
            .id("mode-dropdown-overlay")
            .absolute()
            .top(px(68.0))
            .left(px(20.0))
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
}
