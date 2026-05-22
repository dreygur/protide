//! tRPC Playground — top-level layout and sidebar header rendering

use gpui::{
    deferred, div, prelude::*, px, Context, IntoElement, MouseDownEvent,
    MouseMoveEvent, ParentElement, Styled,
};
use crate::theme;
use crate::components::icons::{icon, ICON_SM, ICON_SEARCH};
use gpui_component::input::Input;
use protide_core::execution::ws::WebSocketExecutor;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_trpc_playground_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let sidebar_w = self.trpc_pg_sidebar_w;

        div()
            .id("trpc-playground")
            .size_full()
            .flex()
            // Left: procedure sidebar
            .child(
                div()
                    .w(px(sidebar_w))
                    .h_full()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .bg(theme.colors.bg_secondary)
                    .border_r_1()
                    .border_color(theme.colors.border)
                    .child(self.render_pg_sidebar_header(cx))
                    .child(self.render_pg_proc_list(cx))
                    .child(self.render_pg_add_row(cx))
            )
            // Drag handle between sidebar and middle
            .child(
                div()
                    .id("trpc-pg-sidebar-handle")
                    .w(px(4.0))
                    .h_full()
                    .flex_none()
                    .cursor_col_resize()
                    .bg(theme.colors.border.opacity(0.0))
                    .hover(|s| s.bg(theme.colors.accent.opacity(0.3)))
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, _, _| {
                        this.trpc_pg_sidebar_drag = Some((f32::from(event.position.x), sidebar_w));
                    }))
            )
            // Middle: params editor + run bar
            .child(self.render_pg_middle(cx))
            // Column divider
            .child(div().w(px(1.0)).h_full().flex_none().bg(theme.colors.border))
            // Right: response viewer
            .child(self.render_pg_right(cx))
            // Sidebar resize overlay (shown while dragging)
            .when(self.trpc_pg_sidebar_drag.is_some(), |el| {
                el.child(deferred(
                    div()
                        .id("trpc-pg-sidebar-drag-overlay")
                        .absolute()
                        .inset_0()
                        .cursor_col_resize()
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                            if let Some((start_x, start_w)) = this.trpc_pg_sidebar_drag {
                                let new_w = (start_w + f32::from(event.position.x) - start_x)
                                    .max(160.0).min(400.0);
                                this.trpc_pg_sidebar_w = new_w;
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.trpc_pg_sidebar_drag = None;
                            cx.notify();
                        })),
                ).with_priority(5))
            })
            .into_any_element()
    }

    pub(super) fn render_pg_sidebar_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let search = self.trpc_pg_search_input.read(cx).value().to_string();
        let q = search.to_lowercase();
        let count = if q.is_empty() {
            self.trpc_pg_procedures.len()
        } else {
            self.trpc_pg_procedures.iter()
                .filter(|p| p.name.to_lowercase().contains(&q))
                .count()
        };

        div()
            .flex_none()
            .flex()
            .flex_col()
            .border_b_1()
            .border_color(theme.colors.border)
            // Title row
            .child(
                div()
                    .h(px(36.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Procedures")
                    )
                    .child(
                        div()
                            .px(px(5.0))
                            .py(px(1.0))
                            .bg(theme.colors.accent.opacity(0.12))
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.accent)
                            .child(count.to_string())
                    )
            )
            // Search row
            .child(
                div()
                    .h(px(30.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_primary)
                    .child(icon(ICON_SEARCH, ICON_SM, theme.colors.text_muted))
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .child(Input::new(&self.trpc_pg_search_input).bordered(false))
                    )
            )
    }
}
