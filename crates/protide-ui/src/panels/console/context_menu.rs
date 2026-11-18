use super::*;

use gpui::{ClipboardItem, FontWeight, MouseButton, div, prelude::*, px};

pub(super) fn render_context_menu(
    entry: ConsoleEntry,
    pos: gpui::Point<gpui::Pixels>,
    theme: &crate::theme::Theme,
    cx: &mut gpui::Context<ConsolePanel>,
) -> impl IntoElement {
    use crate::components::icons::{ICON_COPY, ICON_ELLIPSIS, ICON_SM, icon};
    let x = f32::from(pos.x);
    let y = f32::from(pos.y);
    let has_hint = entry.troubleshoot_hint.is_some();
    let has_error = entry.error.is_some();
    let hint_text = entry.troubleshoot_hint.clone().unwrap_or_default();
    let error_detail = entry.error_details();

    gpui::deferred(
        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(210.0))
            .bg(theme.colors.bg_secondary)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child({
                        let curl = entry.as_curl();
                        let bg = theme.colors.bg_tertiary;
                        let tm = theme.colors.text_muted;
                        let tp = theme.colors.text_primary;
                        div()
                            .id("console-ctx-curl")
                            .w_full()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .gap(px(8.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(bg))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(curl.clone()));
                                this.context_menu = None;
                                cx.notify();
                            }))
                            .child(icon(ICON_COPY, ICON_SM, tm))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(tp)
                                    .child("Copy as cURL"),
                            )
                    })
                    .child({
                        let body = entry.response_body.clone();
                        let bg = theme.colors.bg_tertiary;
                        let tm = theme.colors.text_muted;
                        let tp = theme.colors.text_primary;
                        div()
                            .id("console-ctx-body")
                            .w_full()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .gap(px(8.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(bg))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(body.clone()));
                                this.context_menu = None;
                                cx.notify();
                            }))
                            .child(icon(ICON_ELLIPSIS, ICON_SM, tm))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(tp)
                                    .child("Copy Response"),
                            )
                    })
                    .when(has_error, |el| {
                        let detail = error_detail.clone();
                        let bg = theme.colors.bg_tertiary;
                        let tm = theme.colors.text_muted;
                        let tp = theme.colors.text_primary;
                        el.child(
                            div()
                                .id("console-ctx-err-detail")
                                .w_full()
                                .h(px(28.0))
                                .flex()
                                .items_center()
                                .px(px(12.0))
                                .gap(px(8.0))
                                .cursor_pointer()
                                .hover(move |s| s.bg(bg))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        detail.clone(),
                                    ));
                                    this.context_menu = None;
                                    cx.notify();
                                }))
                                .child(icon(ICON_COPY, ICON_SM, tm))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(tp)
                                        .child("Copy Error Details"),
                                ),
                        )
                    })
                    .when(has_hint, |el| {
                        el.child(
                            div()
                                .h(px(1.0))
                                .mx(px(8.0))
                                .my(px(2.0))
                                .bg(theme.colors.border),
                        )
                    })
                    .when(has_hint, |el| {
                        el.child(
                            div()
                                .id("console-ctx-troubleshoot")
                                .w_full()
                                .px(px(12.0))
                                .py(px(6.0))
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.bg_tertiary))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        hint_text.clone(),
                                    ));
                                    this.context_menu = None;
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            div()
                                                .size(px(12.0))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .bg(theme.colors.warning.opacity(0.2))
                                                .text_size(px(8.0))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(theme.colors.warning)
                                                .child("?"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.0))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(theme.colors.warning)
                                                .child("Troubleshoot"),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(theme.colors.text_muted)
                                        .child("Click to copy steps to clipboard"),
                                ),
                        )
                    }),
            ),
    )
    .with_priority(10)
}
