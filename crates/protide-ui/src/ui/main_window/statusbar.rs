use gpui::{Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, px, prelude::*};
use super::*;

impl MainWindow {
    pub(super) fn render_status_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        let protocol = self.request_panel.read(cx).mode_label();
        let protocol_color = theme.method_color(protocol);

        let response_info = self.response_panel.read(cx).last_response_summary();
        let is_loading = self.response_panel.read(cx).is_loading();

        let sep = || {
            div()
                .w(px(1.0))
                .h(px(10.0))
                .bg(theme.colors.border)
                .mx(px(6.0))
        };

        div()
            .id("status-bar")
            .h(px(22.0))
            .w_full()
            .flex()
            .items_center()
            .flex_shrink_0()
            .px(px(10.0))
            .gap(px(0.0))
            .bg(theme.colors.bg_primary)
            .border_t_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(div().size(px(6.0)).bg(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Local Dev"),
                    ),
            )
            .child(sep())
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(protocol_color)
                    .child(protocol),
            )
            .child(sep())
            .child(if is_loading {
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Sending…"),
                    )
                    .into_any_element()
            } else if let Some((status, _, time_ms, size_bytes)) = response_info {
                let status_color = theme.status_color(status);
                let size_str = if size_bytes >= 1024 * 1024 {
                    format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0))
                } else if size_bytes >= 1024 {
                    format!("{:.1} KB", size_bytes as f64 / 1024.0)
                } else {
                    format!("{} B", size_bytes)
                };
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(status_color)
                            .child(format!("{}", status)),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("·"),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child(format!("{}ms", time_ms)),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("·"),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child(size_str),
                    )
                    .into_any_element()
            } else {
                div()
                    .text_size(px(10.0))
                    .text_color(theme.colors.text_muted)
                    .child("Ready")
                    .into_any_element()
            })
            .child(div().flex_1())
            .child({
                let show_console = self.show_console;
                let count = self.console_panel.read(cx).entry_count();
                div()
                    .id("toggle-console-btn")
                    .h_full()
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .cursor_pointer()
                    .when(show_console, |el| el.bg(theme.colors.accent.opacity(0.12)))
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_console(cx)))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if show_console {
                                theme.colors.accent
                            } else {
                                theme.colors.text_muted
                            })
                            .child("Console")
                    )
                    .when(count > 0, |el| {
                        el.child(
                            div()
                                .px(px(4.0))
                                .py(px(1.0))
                                .bg(theme.colors.accent.opacity(0.15))
                                .text_size(px(9.0))
                                .text_color(theme.colors.accent)
                                .child(SharedString::from(format!("{}", count)))
                        )
                    })
            })
    }
}
