//! tRPC Playground — top-level layout and sidebar header rendering

use gpui::{
    deferred, div, prelude::*, px, Context, IntoElement, MouseDownEvent,
    MouseMoveEvent, ParentElement, Styled,
};
use crate::theme;
use crate::components::icons::{icon, ICON_SM, ICON_FILE, ICON_GLOBE, ICON_SEARCH, ICON_REFRESH};
use gpui_component::input::Input;
use protide_core::execution::ws::WebSocketExecutor;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_trpc_playground_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let sidebar_w = self.trpc.pg_sidebar_w;
        let is_dragging = self.trpc.pg_sidebar_drag.is_some();

        let sidebar_header = self.render_pg_sidebar_header(cx);
        let proc_list = self.render_pg_proc_list(cx);
        let add_row = self.render_pg_add_row(cx);
        let middle = self.render_pg_middle(cx);
        let right = self.render_pg_right(cx);

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
                    .child(sidebar_header)
                    .child(proc_list)
                    .child(add_row)
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
                        this.trpc.pg_sidebar_drag = Some((f32::from(event.position.x), sidebar_w));
                    }))
            )
            // Middle: params editor + run bar
            .child(middle)
            // Column divider
            .child(div().w(px(1.0)).h_full().flex_none().bg(theme.colors.border))
            // Right: response viewer
            .child(right)
            // Sidebar resize overlay (shown while dragging)
            .when(is_dragging, |el| {
                el.child(deferred(
                    div()
                        .id("trpc-pg-sidebar-drag-overlay")
                        .absolute()
                        .inset_0()
                        .cursor_col_resize()
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                            if let Some((start_x, start_w)) = this.trpc.pg_sidebar_drag {
                                let new_w = (start_w + f32::from(event.position.x) - start_x)
                                    .max(160.0).min(400.0);
                                this.trpc.pg_sidebar_w = new_w;
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.trpc.pg_sidebar_drag = None;
                            cx.notify();
                        })),
                ).with_priority(5))
            })
            .into_any_element()
    }

    pub(super) fn render_pg_sidebar_header(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let search = self.trpc.pg_search_input.read(cx).value().to_string();
        let q = search.to_lowercase();
        let count = if q.is_empty() {
            self.trpc.pg_procedures.len()
        } else {
            self.trpc.pg_procedures.iter()
                .filter(|p| p.name.to_lowercase().contains(&q))
                .count()
        };
        let loading = self.trpc.pg_schema_loading;
        let schema_error = self.trpc.pg_schema_error.clone();
        let show_import_url = self.trpc.pg_show_import_url;

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
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Procedures")
                    )
                    .child(
                        div()
                            .px(px(5.0)).py(px(1.0))
                            .bg(theme.colors.accent.opacity(0.12))
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.accent)
                            .child(count.to_string())
                    )
                    .child(div().flex_1())
                    // Fetch Schema button
                    .child(
                        div()
                            .id("trpc-fetch-schema")
                            .px(px(6.0)).py(px(3.0))
                            .flex().items_center().gap(px(4.0))
                            .rounded(px(3.0))
                            .cursor_pointer()
                            .bg(theme.colors.accent.opacity(if loading { 0.05 } else { 0.0 }))
                            .hover(|s| s.bg(theme.colors.accent.opacity(0.12)))
                            .child(icon(ICON_REFRESH, ICON_SM,
                                if loading { theme.colors.text_muted } else { theme.colors.accent }
                            ))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(if loading { theme.colors.text_muted } else { theme.colors.accent })
                                    .child(if loading { "Fetching..." } else { "Fetch" })
                            )
                            .when(!loading, |el| {
                                el.on_click(cx.listener(|this, _, _, cx| {
                                    this.fetch_trpc_schema(cx);
                                }))
                            })
                    )
                    // Import from file button
                    .child(
                        div()
                            .id("trpc-import-file")
                            .px(px(5.0)).py(px(3.0))
                            .flex().items_center()
                            .rounded(px(3.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.hover_overlay))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.import_trpc_from_file(cx);
                            }))
                            .child(icon(ICON_FILE, ICON_SM, theme.colors.text_secondary))
                    )
                    // Import from URL button (toggles URL row)
                    .child(
                        div()
                            .id("trpc-import-url-toggle")
                            .px(px(5.0)).py(px(3.0))
                            .flex().items_center()
                            .rounded(px(3.0))
                            .cursor_pointer()
                            .when(show_import_url, |el| el.bg(theme.colors.accent.opacity(0.1)))
                            .hover(|s| s.bg(theme.colors.hover_overlay))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.trpc.pg_show_import_url = !this.trpc.pg_show_import_url;
                                cx.notify();
                            }))
                            .child(icon(ICON_GLOBE, ICON_SM,
                                if show_import_url { theme.colors.accent } else { theme.colors.text_secondary }
                            ))
                    )
            )
            // URL import row
            .when(show_import_url, |el| {
                el.child(
                    div()
                        .h(px(30.0))
                        .px(px(6.0))
                        .flex().items_center().gap(px(4.0))
                        .border_b_1()
                        .border_color(theme.colors.accent.opacity(0.3))
                        .bg(theme.colors.bg_tertiary)
                        .child(
                            div()
                                .flex_1().h_full()
                                .overflow_hidden()
                                .child(Input::new(&self.trpc.pg_import_url_input).bordered(false))
                        )
                        .child(
                            div()
                                .id("trpc-import-url-btn")
                                .px(px(8.0)).h(px(22.0))
                                .flex().items_center()
                                .bg(theme.colors.accent.opacity(if loading { 0.04 } else { 0.08 }))
                                .border_1()
                                .border_color(theme.colors.accent.opacity(0.3))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(if loading { theme.colors.text_muted } else { theme.colors.accent })
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.accent.opacity(0.16)))
                                .when(!loading, |el| {
                                    el.on_click(cx.listener(|this, _, _, cx| {
                                        this.import_trpc_from_url(cx);
                                    }))
                                })
                                .child(if loading { "Importing…" } else { "Import" })
                        )
                )
            })
            // Search row
            .child(
                div()
                    .h(px(30.0))
                    .px(px(8.0))
                    .flex().items_center().gap(px(6.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_primary)
                    .child(icon(ICON_SEARCH, ICON_SM, theme.colors.text_muted))
                    .child(
                        div()
                            .flex_1().h_full()
                            .overflow_hidden()
                            .child(Input::new(&self.trpc.pg_search_input).bordered(false))
                    )
            )
            // Error banner
            .when_some(schema_error, |el, err| {
                el.child(
                    div()
                        .px(px(10.0)).py(px(6.0))
                        .text_size(px(10.0))
                        .text_color(gpui::rgb(0xf87171))
                        .bg(gpui::rgba(0xf8717114))
                        .border_b_1()
                        .border_color(gpui::rgba(0xf8717140))
                        .child(err)
                )
            })
            .into_any_element()
    }
}
