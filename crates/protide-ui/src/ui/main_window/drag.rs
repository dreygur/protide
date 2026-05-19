use gpui::{Context, IntoElement, MouseButton, ParentElement, Styled, div, px, prelude::*};
use super::*;

impl MainWindow {
    pub(super) fn render_collapsed_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        div()
            .id("sidebar-collapsed-strip")
            .w(px(32.0))
            .h_full()
            .flex_shrink_0()
            .bg(theme.colors.bg_secondary)
            .border_r_1()
            .border_color(theme.colors.border)
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.0))
            .pt(px(8.0))
            .child(
                div()
                    .id("collapse-toggle")
                    .w(px(28.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_sidebar(cx)))
                    .child(icon(ICON_MENU, ICON_MD, theme.colors.text_muted)),
            )
            .child(
                div()
                    .w_full()
                    .h(px(1.0))
                    .bg(theme.colors.border)
                    .mx_auto()
                    .mt(px(2.0))
                    .mb(px(2.0)),
            )
            .child({
                let explorer = self.explorer.clone();
                div()
                    .id("collapsed-collections")
                    .w(px(28.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_sidebar(cx);
                        explorer.update(cx, |p, cx| p.expand_section_collections(cx));
                    }))
                    .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_muted))
            })
            .child({
                let explorer = self.explorer.clone();
                div()
                    .id("collapsed-history")
                    .w(px(28.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_sidebar(cx);
                        explorer.update(cx, |p, cx| p.expand_section_history(cx));
                    }))
                    .child(icon(ICON_REFRESH, ICON_MD, theme.colors.text_muted))
            })
            .child({
                let explorer = self.explorer.clone();
                div()
                    .id("collapsed-env")
                    .w(px(28.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_sidebar(cx);
                        explorer.update(cx, |p, cx| p.expand_section_env(cx));
                    }))
                    .child(icon(ICON_SETTINGS, ICON_MD, theme.colors.text_muted))
            })
    }

    pub(super) fn render_drag_overlay(&mut self, is_col_drag: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("resize-drag-overlay")
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .when(is_col_drag, |el| el.cursor_col_resize())
            .when(!is_col_drag, |el| el.cursor_row_resize())
            .on_mouse_move(cx.listener(
                |this, event: &gpui::MouseMoveEvent, _window, cx| {
                    let mouse_x = f32::from(event.position.x);
                    let mouse_y = f32::from(event.position.y);
                    if let Some((start_x, start_w)) = this.drag_sidebar {
                        this.sidebar_width = (start_w + mouse_x - start_x).clamp(150.0, 600.0);
                        cx.notify();
                    }
                    if let Some((start_y, start_h)) = this.drag_response {
                        this.request_height = (start_h + mouse_y - start_y).clamp(150.0, 800.0);
                        cx.notify();
                    }
                    if let Some((start_x, start_w)) = this.drag_mock_server {
                        this.mock_server_width = (start_w - (mouse_x - start_x)).clamp(200.0, 700.0);
                        cx.notify();
                    }
                    if let Some((start_x, start_w)) = this.drag_codegen {
                        this.codegen_panel_width = (start_w - (mouse_x - start_x)).clamp(250.0, 800.0);
                        cx.notify();
                    }
                    if let Some((start_x, start_w)) = this.drag_docs {
                        this.docs_width = (start_w - (mouse_x - start_x)).clamp(260.0, 800.0);
                        cx.notify();
                    }
                    if let Some((start_y, start_h)) = this.drag_console {
                        this.console_height = (start_h - (mouse_y - start_y)).clamp(80.0, 500.0);
                        cx.notify();
                    }
                },
            ))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _window, cx| {
                    if this.drag_sidebar.take().is_some() {
                        crate::prefs::set_f32("main.sidebar_width", this.sidebar_width);
                    }
                    if this.drag_response.take().is_some() {
                        crate::prefs::set_f32("main.request_height", this.request_height);
                    }
                    if this.drag_mock_server.take().is_some() {
                        crate::prefs::set_f32("main.mock_server_width", this.mock_server_width);
                    }
                    if this.drag_codegen.take().is_some() {
                        crate::prefs::set_f32("main.codegen_panel_width", this.codegen_panel_width);
                    }
                    if this.drag_docs.take().is_some() {
                        crate::prefs::set_f32("main.docs_width", this.docs_width);
                    }
                    this.drag_console.take();
                    cx.notify();
                }),
            )
    }

}
