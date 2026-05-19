use gpui::{Context, FontWeight, IntoElement, MouseButton, ParentElement, Styled, div, px, prelude::*};
use super::*;

impl MainWindow {
    pub(super) fn render_help_overlay(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        let shortcuts: &[(&str, &str, &str)] = &[
            ("Request", "Ctrl+Enter", "Send request"),
            ("Request", "Ctrl+S", "Save request"),
            ("View", "Ctrl+B", "Toggle sidebar"),
            ("View", "Ctrl+Shift+M", "Toggle mock server"),
            ("View", "Ctrl+Shift+D", "Toggle API explorer"),
            ("Help", "F1", "Show keyboard shortcuts"),
            ("Help", "Ctrl+Shift+A", "About Protide"),
            ("General", "Escape", "Close dialog / overlay"),
        ];

        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.bg_primary.opacity(0.7))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.show_help = false;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w(px(480.0))
                    .bg(theme.colors.bg_elevated)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(14.0))
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("Keyboard Shortcuts"),
                            )
                            .child(
                                div()
                                    .id("help-close")
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(theme.colors.text_primary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_help = false;
                                        cx.notify();
                                    }))
                                    .child("✕"),
                            ),
                    )
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(12.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(shortcuts.iter().map(|(group, key, desc)| {
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .py(px(5.0))
                                    .border_b_1()
                                    .border_color(theme.colors.border.opacity(0.4))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(10.0))
                                            .child(
                                                div()
                                                    .w(px(60.0))
                                                    .text_size(px(10.0))
                                                    .text_color(theme.colors.text_muted)
                                                    .child(*group),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(theme.colors.text_secondary)
                                                    .child(*desc),
                                            ),
                                    )
                                    .child(div().flex().items_center().gap(px(3.0)).children(
                                        key.split('+').map(|k| {
                                            div()
                                                .px(px(7.0))
                                                .py(px(3.0))
                                                .bg(theme.colors.bg_tertiary)
                                                .border_1()
                                                .border_color(theme.colors.border)
                                                .text_size(px(10.0))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.colors.text_primary)
                                                .child(k.trim())
                                        }),
                                    ))
                            })),
                    )
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(10.0))
                            .border_t_1()
                            .border_color(theme.colors.border)
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Press Escape or click outside to close"),
                    ),
            )
    }

    pub(super) fn render_about_overlay(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.bg_primary.opacity(0.7))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.show_about = false;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w(px(360.0))
                    .bg(theme.colors.bg_elevated)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(14.0))
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("About"),
                            )
                            .child(
                                div()
                                    .id("about-close")
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(theme.colors.text_primary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_about = false;
                                        cx.notify();
                                    }))
                                    .child("✕"),
                            ),
                    )
                    .child(
                        div()
                            .px(px(28.0))
                            .py(px(24.0))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(14.0))
                            .child(
                                div()
                                    .size(px(56.0))
                                    .bg(theme.colors.accent)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(28.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.colors.bg_primary)
                                            .child("P"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .text_size(px(20.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.colors.text_primary)
                                            .child("Protide"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child(format!(
                                                "Version {}",
                                                env!("CARGO_PKG_VERSION")
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .text_center()
                                    .child("Free and open-source API testing tool"),
                            )
                            .child(div().w_full().h(px(1.0)).bg(theme.colors.border))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(3.0))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("Developed by"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_primary)
                                            .child("Rakibul Yeasin"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(10.0))
                            .border_t_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .id("about-ok")
                                    .px(px(28.0))
                                    .py(px(7.0))
                                    .bg(theme.colors.accent)
                                    .text_color(theme.colors.bg_primary)
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.accent_hover))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_about = false;
                                        cx.notify();
                                    }))
                                    .child("Close"),
                            ),
                    ),
            )
    }
}
