use gpui::{Context, IntoElement, MouseButton, ParentElement, Styled, div, px, prelude::*};
use super::MainWindow;
use crate::theme;

impl MainWindow {
    pub(super) fn render_sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .flex()
            .h_full()
            .child(
                div()
                    .w(px(self.sidebar_width))
                    .h_full()
                    .flex_shrink_0()
                    .bg(theme.colors.bg_secondary)
                    .overflow_hidden()
                    .child(self.explorer.clone()),
            )
            .child(
                div()
                    .id("sidebar-resize-handle")
                    .w(px(4.0))
                    .h_full()
                    .flex_shrink_0()
                    .border_l_1()
                    .border_color(theme.colors.border)
                    .cursor_col_resize()
                    .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                            this.drag_sidebar = Some((f32::from(event.position.x), this.sidebar_width));
                        }),
                    ),
            )
    }
}
