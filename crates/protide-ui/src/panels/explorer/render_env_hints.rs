use super::*;
use gpui::{Context, IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled, div, px};

impl ExplorerPanel {
    pub(super) fn render_env_col_drag_handle(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let start_w = self.env_col_key_w;
        div()
            .id("env-col-drag-handle")
            .w(px(4.0))
            .self_stretch()
            .cursor_col_resize()
            .bg(theme.colors.border.opacity(0.3))
            .hover(|s| s.bg(theme.colors.accent.opacity(0.5)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.env_col_drag = Some((f32::from(event.position.x), start_w));
                    cx.notify();
                }),
            )
    }

    pub(super) fn render_env_no_selection_hint(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        div()
            .w_full()
            .py(px(16.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .size(px(36.0))
                    .bg(theme.colors.bg_tertiary)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon(ICON_SETTINGS, ICON_MD, theme.colors.text_muted)),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.colors.text_muted)
                    .child("Select an environment to edit"),
            )
    }

    pub(super) fn render_env_usage_hint(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        div()
            .w_full()
            .mt(px(4.0))
            .px(px(8.0))
            .py(px(6.0))
            .bg(theme.colors.bg_tertiary.opacity(0.5))
            .flex()
            .items_start()
            .gap(px(6.0))
            .child(icon(ICON_INFO, ICON_MD, theme.colors.text_muted))
            .child(
                div()
                    .flex_1()
                    .text_size(px(10.0))
                    .text_color(theme.colors.text_muted)
                    .child("Use {{var}} in URL, headers, body, or auth"),
            )
    }
}
