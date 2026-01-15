//! Mock server panel UI

use gpui::{
    div, prelude::*, px, Context, FocusHandle, IntoElement, ParentElement,
    Render, Styled, Window,
};

use crate::mock_server::{HttpMethod, MockResponse, MockRoute, MockServer};
use crate::theme;

/// Mock server panel
pub struct MockServerPanel {
    server: MockServer,
    #[allow(dead_code)]
    focus: FocusHandle,
    new_route_method: HttpMethod,
    new_route_status: u16,
}

impl MockServerPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            server: MockServer::new(8080),
            focus: cx.focus_handle(),
            new_route_method: HttpMethod::Get,
            new_route_status: 200,
        }
    }

    fn toggle_server(&mut self, cx: &mut Context<Self>) {
        if self.server.is_running() {
            self.server.stop();
        } else {
            let _ = self.server.start();
        }
        cx.notify();
    }

    fn add_route(&mut self, cx: &mut Context<Self>) {
        let response = MockResponse::new(self.new_route_status, r#"{"message":"mock response"}"#)
            .with_header("Content-Type", "application/json");
        let route = MockRoute::new(self.new_route_method, "/api/mock", response);
        self.server.add_route(route);
        cx.notify();
    }

    fn remove_route(&mut self, index: usize, cx: &mut Context<Self>) {
        self.server.remove_route(index);
        cx.notify();
    }

    fn set_method(&mut self, method: HttpMethod, cx: &mut Context<Self>) {
        self.new_route_method = method;
        cx.notify();
    }

    fn set_status(&mut self, status: u16, cx: &mut Context<Self>) {
        self.new_route_status = status;
        cx.notify();
    }
}

impl Render for MockServerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_running = self.server.is_running();
        let base_url = self.server.base_url();
        let routes = self.server.routes();
        let new_method = self.new_route_method;
        let new_status = self.new_route_status;

        div()
            .id("mock-server-panel")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_primary)
            .border_l_1()
            .border_color(theme.colors.border)
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.text_primary)
                            .child("Mock Server")
                    )
                    .child(
                        div()
                            .id("toggle-server")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(if is_running { theme.colors.status_client_error } else { theme.colors.status_success })
                            .text_color(theme.colors.bg_primary)
                            .text_sm()
                            .child(if is_running { "Stop" } else { "Start" })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_server(cx);
                            }))
                    )
            )
            // Status bar
            .child(
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_secondary)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w_2()
                                    .h_2()
                                    .rounded_full()
                                    .bg(if is_running { theme.colors.status_success } else { theme.colors.text_muted })
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.colors.text_secondary)
                                    .child(base_url.unwrap_or_else(|| "Not running".to_string()))
                            )
                    )
            )
            // Routes list
            .child(
                div()
                    .id("routes-list")
                    .flex_1()
                    .overflow_scroll()
                    .child(
                        div()
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .when(routes.is_empty(), |el| {
                                el.child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.colors.text_muted)
                                        .child("No routes configured")
                                )
                            })
                            .children((0..routes.len()).map(|i| {
                                let route = &routes[i];
                                let method_color = match route.method {
                                    HttpMethod::Get => theme.colors.method_get,
                                    HttpMethod::Post => theme.colors.method_post,
                                    HttpMethod::Put => theme.colors.method_put,
                                    HttpMethod::Patch => theme.colors.method_patch,
                                    HttpMethod::Delete => theme.colors.method_delete,
                                    _ => theme.colors.text_secondary,
                                };
                                div()
                                    .id(("route", i))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(theme.colors.bg_secondary)
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::BOLD)
                                                    .text_color(method_color)
                                                    .child(route.method.as_str())
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.colors.text_primary)
                                                    .child(route.path.clone())
                                            )
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .px_2()
                                                    .py_px()
                                                    .rounded_sm()
                                                    .bg(status_color(route.response.status, &theme.colors))
                                                    .text_color(theme.colors.bg_primary)
                                                    .child(format!("{}", route.response.status))
                                            )
                                            .child(
                                                div()
                                                    .id(("delete", i))
                                                    .text_xs()
                                                    .cursor_pointer()
                                                    .text_color(theme.colors.status_client_error)
                                                    .child("x")
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.remove_route(i, cx);
                                                    }))
                                            )
                                    )
                            }))
                    )
            )
            // Add route form
            .child(
                div()
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.text_primary)
                            .child("Add Route")
                    )
                    // Method selector
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id("method-get")
                                    .px_2().py_1().rounded_sm().cursor_pointer().text_xs()
                                    .bg(if new_method == HttpMethod::Get { theme.colors.accent } else { theme.colors.bg_secondary })
                                    .text_color(if new_method == HttpMethod::Get { theme.colors.bg_primary } else { theme.colors.text_secondary })
                                    .child("GET")
                                    .on_click(cx.listener(|this, _, _, cx| this.set_method(HttpMethod::Get, cx)))
                            )
                            .child(
                                div()
                                    .id("method-post")
                                    .px_2().py_1().rounded_sm().cursor_pointer().text_xs()
                                    .bg(if new_method == HttpMethod::Post { theme.colors.accent } else { theme.colors.bg_secondary })
                                    .text_color(if new_method == HttpMethod::Post { theme.colors.bg_primary } else { theme.colors.text_secondary })
                                    .child("POST")
                                    .on_click(cx.listener(|this, _, _, cx| this.set_method(HttpMethod::Post, cx)))
                            )
                            .child(
                                div()
                                    .id("method-put")
                                    .px_2().py_1().rounded_sm().cursor_pointer().text_xs()
                                    .bg(if new_method == HttpMethod::Put { theme.colors.accent } else { theme.colors.bg_secondary })
                                    .text_color(if new_method == HttpMethod::Put { theme.colors.bg_primary } else { theme.colors.text_secondary })
                                    .child("PUT")
                                    .on_click(cx.listener(|this, _, _, cx| this.set_method(HttpMethod::Put, cx)))
                            )
                            .child(
                                div()
                                    .id("method-delete")
                                    .px_2().py_1().rounded_sm().cursor_pointer().text_xs()
                                    .bg(if new_method == HttpMethod::Delete { theme.colors.accent } else { theme.colors.bg_secondary })
                                    .text_color(if new_method == HttpMethod::Delete { theme.colors.bg_primary } else { theme.colors.text_secondary })
                                    .child("DELETE")
                                    .on_click(cx.listener(|this, _, _, cx| this.set_method(HttpMethod::Delete, cx)))
                            )
                    )
                    // Status selector
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_xs().text_color(theme.colors.text_secondary).child("Status:"))
                            .child(
                                div()
                                    .id("status-200")
                                    .px_2().py_1().rounded_sm().cursor_pointer().text_xs()
                                    .bg(if new_status == 200 { theme.colors.status_success } else { theme.colors.bg_secondary })
                                    .text_color(if new_status == 200 { theme.colors.bg_primary } else { theme.colors.text_secondary })
                                    .child("200")
                                    .on_click(cx.listener(|this, _, _, cx| this.set_status(200, cx)))
                            )
                            .child(
                                div()
                                    .id("status-400")
                                    .px_2().py_1().rounded_sm().cursor_pointer().text_xs()
                                    .bg(if new_status == 400 { theme.colors.status_client_error } else { theme.colors.bg_secondary })
                                    .text_color(if new_status == 400 { theme.colors.bg_primary } else { theme.colors.text_secondary })
                                    .child("400")
                                    .on_click(cx.listener(|this, _, _, cx| this.set_status(400, cx)))
                            )
                            .child(
                                div()
                                    .id("status-404")
                                    .px_2().py_1().rounded_sm().cursor_pointer().text_xs()
                                    .bg(if new_status == 404 { theme.colors.status_client_error } else { theme.colors.bg_secondary })
                                    .text_color(if new_status == 404 { theme.colors.bg_primary } else { theme.colors.text_secondary })
                                    .child("404")
                                    .on_click(cx.listener(|this, _, _, cx| this.set_status(404, cx)))
                            )
                            .child(
                                div()
                                    .id("status-500")
                                    .px_2().py_1().rounded_sm().cursor_pointer().text_xs()
                                    .bg(if new_status == 500 { theme.colors.status_server_error } else { theme.colors.bg_secondary })
                                    .text_color(if new_status == 500 { theme.colors.bg_primary } else { theme.colors.text_secondary })
                                    .child("500")
                                    .on_click(cx.listener(|this, _, _, cx| this.set_status(500, cx)))
                            )
                    )
                    // Add button
                    .child(
                        div()
                            .id("add-route-btn")
                            .mt_2()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(theme.colors.accent)
                            .text_color(theme.colors.bg_primary)
                            .text_sm()
                            .text_center()
                            .child("Add Route")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.add_route(cx);
                            }))
                    )
            )
    }
}

fn status_color(status: u16, colors: &theme::Colors) -> gpui::Hsla {
    match status {
        200..=299 => colors.status_success,
        300..=399 => colors.accent,
        400..=499 => colors.status_client_error,
        _ => colors.status_server_error,
    }
}
