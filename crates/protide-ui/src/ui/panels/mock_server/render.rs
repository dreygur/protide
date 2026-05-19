use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div, prelude::*};
use protide_core::mock_server::HttpMethod;
use super::*;

pub(super) fn status_color(status: u16, colors: &crate::theme::Colors) -> gpui::Hsla {
    match status {
        200..=299 => colors.status_success,
        300..=399 => colors.accent,
        400..=499 => colors.status_client_error,
        _ => colors.status_server_error,
    }
}

impl Render for MockServerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_running = self.server.is_running();
        let base_url = self.server.base_url();
        let routes = self.server.routes();

        div()
            .id("mock-server-panel")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_primary)
            .border_l_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(div().text_sm().text_color(theme.colors.text_primary).child("Mock Server"))
                    .child(
                        div()
                            .id("toggle-server")
                            .px_3().py_1()
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
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_server(cx)))
                    )
            )
            .child(
                div()
                    .px_3().py_2()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_secondary)
                    .child(
                        div()
                            .flex().items_center().gap_2()
                            .child(div().w_2().h_2().bg(if is_running { theme.colors.status_success } else { theme.colors.text_muted }))
                            .child(div().text_xs().text_color(theme.colors.text_secondary)
                                .child(base_url.unwrap_or_else(|| "Not running".to_string())))
                    )
            )
            .child(
                div()
                    .id("routes-list")
                    .flex_1()
                    .w_full()
                    .overflow_scroll()
                    .child(
                        div()
                            .p_3().flex().flex_col().gap_2()
                            .when(routes.is_empty(), |el| {
                                el.child(div().text_sm().text_color(theme.colors.text_muted).child("No routes configured"))
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
                                    .flex().items_center().justify_between()
                                    .px_2().py_1()
                                    .bg(theme.colors.bg_secondary)
                                    .border_1().border_color(theme.colors.border)
                                    .child(
                                        div().flex().items_center().gap_2()
                                            .child(div().text_xs().font_weight(gpui::FontWeight::BOLD).text_color(method_color).child(route.method.as_str()))
                                            .child(div().text_sm().text_color(theme.colors.text_primary).child(route.path.clone()))
                                    )
                                    .child(
                                        div().flex().items_center().gap_2()
                                            .child(
                                                div().text_xs().px_2().py_px()
                                                    .bg(if route.is_proxy() { theme.colors.accent } else { status_color(route.response.status, &theme.colors) })
                                                    .text_color(theme.colors.bg_primary)
                                                    .child(if route.is_proxy() {
                                                        format!("→ {}", route.proxy_target.as_deref().unwrap_or(""))
                                                    } else {
                                                        format!("{}", route.response.status)
                                                    })
                                            )
                                            .child(
                                                div().id(("delete", i)).text_xs().cursor_pointer()
                                                    .text_color(theme.colors.status_client_error)
                                                    .child("x")
                                                    .on_click(cx.listener(move |this, _, _, cx| this.remove_route(i, cx)))
                                            )
                                    )
                            }))
                    )
            )
            .child(self.render_add_form(cx))
    }
}
