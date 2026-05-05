//! Mock server panel UI

use gpui::{
    div, prelude::*, px, Context, Entity, FocusHandle, IntoElement, ParentElement,
    Render, Styled, WeakEntity, Window,
};

use protide_core::mock_server::{HttpMethod, MockResponse, MockRoute, MockServer};
use crate::theme;
use crate::ui::components::{modal::ModalState, TextInput, TextInputStyle};
use crate::ui::main_window::MainWindow;

/// Mock server panel
pub struct MockServerPanel {
    server: MockServer,
    #[allow(dead_code)]
    focus: FocusHandle,
    new_route_method: HttpMethod,
    status_input: Entity<TextInput>,
    proxy_path_input: Entity<TextInput>,
    proxy_target_input: Entity<TextInput>,
    main_window: WeakEntity<MainWindow>,
}

impl MockServerPanel {
    pub fn new(cx: &mut Context<Self>, main_window: WeakEntity<MainWindow>) -> Self {
        let status_input = cx.new(|cx| TextInput::new(cx, "mock-status").text("200").style(TextInputStyle::compact()).placeholder("200"));
        let proxy_path_input = cx.new(|cx| TextInput::new(cx, "proxy-path").placeholder("/api/*"));
        let proxy_target_input = cx.new(|cx| TextInput::new(cx, "proxy-target").placeholder("https://api.example.com"));
        Self {
            server: MockServer::new(8080),
            focus: cx.focus_handle(),
            new_route_method: HttpMethod::Get,
            status_input,
            proxy_path_input,
            proxy_target_input,
            main_window,
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
        let status = self.status_input.read(cx).get_text().trim().parse::<u16>().unwrap_or(200);
        let response = MockResponse::new(status, r#"{"message":"mock response"}"#)
            .with_header("Content-Type", "application/json");
        let route = MockRoute::new(self.new_route_method, "/api/mock", response);
        self.server.add_route(route);
        cx.notify();
    }

    fn add_proxy_route(&mut self, cx: &mut Context<Self>) {
        let raw_path = self.proxy_path_input.read(cx).get_text().to_string();
        let raw_target = self.proxy_target_input.read(cx).get_text().to_string();

        let path = if raw_path.trim().is_empty() { "/api/*".to_string() } else { raw_path.trim().to_string() };
        let target = if raw_target.trim().is_empty() { "https://api.example.com".to_string() } else { raw_target.trim().to_string() };

        let route = MockRoute::proxy(self.new_route_method, &path, &target);
        self.server.add_route(route);

        let modal = ModalState::info(
            "Proxy Route Added",
            format!("Proxy route added: {} {} → {}", self.new_route_method.as_str(), path, target),
        );
        if let Some(win) = self.main_window.upgrade() {
            win.update(cx, |win, cx| win.show_modal(modal, cx));
        }
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
}

impl Render for MockServerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_running = self.server.is_running();
        let base_url = self.server.base_url();
        let routes = self.server.routes();
        let new_method = self.new_route_method;

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
                            .cursor_pointer()
                            .border_1()
                            .when(is_running, |el| el
                                .border_color(theme.colors.status_client_error.opacity(0.5))
                                .bg(theme.colors.status_client_error.opacity(0.1))
                                .text_color(theme.colors.status_client_error)
                            )
                            .when(!is_running, |el| el
                                .border_color(theme.colors.accent.opacity(0.5))
                                .bg(theme.colors.accent.opacity(0.1))
                                .text_color(theme.colors.accent)
                            )
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
                    .w_full()
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
                                                    .bg(if route.is_proxy() { theme.colors.accent } else { status_color(route.response.status, &theme.colors) })
                                                    .text_color(theme.colors.bg_primary)
                                                    .child(if route.is_proxy() {
                                                        format!("→ {}", route.proxy_target.as_deref().unwrap_or(""))
                                                    } else {
                                                        format!("{}", route.response.status)
                                                    })
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
                            .flex_wrap()
                            .items_center()
                            .gap_1()
                            .children(HttpMethod::all().iter().copied().enumerate().map(|(i, m)| {
                                let is_selected = new_method == m;
                                let color = match m {
                                    HttpMethod::Get => theme.colors.method_get,
                                    HttpMethod::Post => theme.colors.method_post,
                                    HttpMethod::Put => theme.colors.method_put,
                                    HttpMethod::Patch => theme.colors.method_patch,
                                    HttpMethod::Delete => theme.colors.method_delete,
                                    _ => theme.colors.text_secondary,
                                };
                                div()
                                    .id(("method-btn", i))
                                    .px_2().py_1().cursor_pointer().text_xs().border_1()
                                    .when(is_selected, |el| el.border_color(color.opacity(0.6)).bg(color.opacity(0.12)).text_color(color))
                                    .when(!is_selected, |el| el.border_color(theme.colors.border).bg(theme.colors.bg_secondary).text_color(theme.colors.text_secondary))
                                    .child(m.as_str())
                                    .on_click(cx.listener(move |this, _, _, cx| this.set_method(m, cx)))
                            }))
                    )
                    // Status code input
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_xs().text_color(theme.colors.text_secondary).child("Status:"))
                            .child(
                                div()
                                    .w(px(80.0))
                                    .child(self.status_input.clone())
                            )
                    )
                    // Proxy route inputs
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_muted)
                                    .child("PROXY PATH")
                            )
                            .child(self.proxy_path_input.clone())
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_muted)
                                    .child("TARGET URL")
                            )
                            .child(self.proxy_target_input.clone())
                    )
                    // Buttons row
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                div()
                                    .id("add-route-btn")
                                    .flex_1()
                                    .px_3()
                                    .py_1()
                                    .cursor_pointer()
                                    .border_1()
                                    .border_color(theme.colors.accent.opacity(0.5))
                                    .bg(theme.colors.accent.opacity(0.1))
                                    .hover(|s| s.bg(theme.colors.accent.opacity(0.18)))
                                    .text_color(theme.colors.accent)
                                    .text_sm()
                                    .text_center()
                                    .child("Add Mock Route")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.add_route(cx);
                                    }))
                            )
                            .child(
                                div()
                                    .id("add-proxy-btn")
                                    .flex_1()
                                    .px_3()
                                    .py_1()
                                    .cursor_pointer()
                                    .bg(theme.colors.bg_tertiary)
                                    .text_color(theme.colors.text_primary)
                                    .text_sm()
                                    .text_center()
                                    .child("Add Proxy Route")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.add_proxy_route(cx);
                                    }))
                            )
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
