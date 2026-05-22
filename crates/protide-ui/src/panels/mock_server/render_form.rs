use gpui::{Context, FontWeight, IntoElement, ParentElement, Styled, div, px, prelude::*};
use gpui_component::input::Input;
use gpui_component::Sizable;
use protide_core::mock_server::HttpMethod;
use super::*;

impl MockServerPanel {
    pub(super) fn render_add_form(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_recording = self.server.is_recording();
        let new_method = self.new_route_method;

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
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.colors.text_primary)
                    .child("Add Route")
            )
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_xs().text_color(theme.colors.text_secondary).child("Status:"))
                    .child(div().w(px(80.0)).child(Input::new(&self.status_input).with_size(gpui_component::Size::Small)))
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.text_muted)
                            .child("PROXY PATH")
                    )
                    .child(Input::new(&self.proxy_path_input))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.text_muted)
                            .child("TARGET URL")
                    )
                    .child(Input::new(&self.proxy_target_input))
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_muted)
                                    .child("RECORD TARGET")
                            )
                            .child(
                                div()
                                    .id("toggle-record-btn")
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .text_size(px(9.0))
                                    .cursor_pointer()
                                    .border_1()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .when(is_recording, |el| el
                                        .border_color(theme.colors.status_server_error.opacity(0.5))
                                        .bg(theme.colors.status_server_error.opacity(0.1))
                                        .text_color(theme.colors.status_server_error)
                                    )
                                    .when(!is_recording, |el| el
                                        .border_color(theme.colors.border)
                                        .bg(theme.colors.bg_secondary)
                                        .text_color(theme.colors.text_secondary)
                                    )
                                    .child(if is_recording { "● REC" } else { "Record" })
                                    .on_click(cx.listener(|this, _, _, cx| this.toggle_record_mode(cx)))
                            )
                    )
                    .child(Input::new(&self.record_target_input))
                    .when(is_recording, |el| {
                        el.child(
                            div()
                                .id("import-recorded-btn")
                                .w_full()
                                .px_3().py_1()
                                .cursor_pointer()
                                .border_1()
                                .border_color(theme.colors.accent.opacity(0.4))
                                .bg(theme.colors.accent.opacity(0.08))
                                .hover(|s| s.bg(theme.colors.accent.opacity(0.15)))
                                .text_color(theme.colors.accent)
                                .text_xs()
                                .text_center()
                                .child("Import Recorded Routes")
                                .on_click(cx.listener(|this, _, _, cx| this.import_recorded(cx)))
                        )
                    })
            )
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
                            .px_3().py_1()
                            .cursor_pointer()
                            .border_1()
                            .border_color(theme.colors.accent.opacity(0.5))
                            .bg(theme.colors.accent.opacity(0.1))
                            .hover(|s| s.bg(theme.colors.accent.opacity(0.18)))
                            .text_color(theme.colors.accent)
                            .text_sm()
                            .text_center()
                            .child("Add Mock Route")
                            .on_click(cx.listener(|this, _, _, cx| this.add_route(cx)))
                    )
                    .child(
                        div()
                            .id("add-proxy-btn")
                            .flex_1()
                            .px_3().py_1()
                            .cursor_pointer()
                            .bg(theme.colors.bg_tertiary)
                            .text_color(theme.colors.text_primary)
                            .text_sm()
                            .text_center()
                            .child("Add Proxy Route")
                            .on_click(cx.listener(|this, _, _, cx| this.add_proxy_route(cx)))
                    )
            )
    }
}
