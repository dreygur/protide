use super::*;
use gpui::{Context, IntoElement, MouseButton, ParentElement, Pixels, Point, Styled, div, px};

impl ExplorerPanel {
    /// Render context menu for collection item
    pub(super) fn render_context_menu(
        &self,
        path: PathBuf,
        position: Point<Pixels>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let path_for_rename = path.clone();
        let path_for_delete = path.clone();
        let path_for_run = path.clone();
        let path_for_export = path.clone();
        let path_for_clone = path.clone();
        let is_folder = path.is_dir();

        let extra_h = if is_folder { 56.0 } else { 28.0 };
        const MENU_W: f32 = 160.0;
        let menu_h = 64.0 + extra_h;
        let x = f32::from(position.x) - f32::from(self.panel_bounds.origin.x);
        let y = f32::from(position.y) - f32::from(self.panel_bounds.origin.y);
        let panel_w = f32::from(self.panel_bounds.size.width);
        let panel_h = f32::from(self.panel_bounds.size.height);
        let left = if x + MENU_W > panel_w {
            px((x - MENU_W).max(0.0))
        } else {
            px(x)
        };
        let top = if y + menu_h > panel_h {
            px((y - menu_h).max(0.0))
        } else {
            px(y)
        };

        div()
            .absolute()
            .left(left)
            .top(top)
            .w(px(MENU_W))
            .bg(theme.colors.bg_secondary)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| {
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .when(is_folder, |el| {
                        el.child(
                            div()
                                .id("context-menu-run")
                                .w_full()
                                .h(px(28.0))
                                .flex()
                                .items_center()
                                .px(px(12.0))
                                .gap(px(8.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.bg_tertiary))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        this.run_collection_folder(path_for_run.clone(), cx);
                                    }),
                                )
                                .child(icon(ICON_PLAY, ICON_MD, theme.colors.status_success))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_primary)
                                        .child("Run Collection"),
                                ),
                        )
                        .child(
                            div()
                                .id("context-menu-export-oas")
                                .w_full()
                                .h(px(28.0))
                                .flex()
                                .items_center()
                                .px(px(12.0))
                                .gap(px(8.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.bg_tertiary))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        this.export_openapi_folder(path_for_export.clone(), cx);
                                    }),
                                )
                                .child(icon(ICON_LINK, ICON_MD, theme.colors.accent))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_primary)
                                        .child("Export OpenAPI"),
                                ),
                        )
                    })
                    .when(!is_folder, |el| {
                        el.child(
                            div()
                                .id("context-menu-clone")
                                .w_full()
                                .h(px(28.0))
                                .flex()
                                .items_center()
                                .px(px(12.0))
                                .gap(px(8.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.bg_tertiary))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        this.context_menu = None;
                                        this.clone_file(path_for_clone.clone(), cx);
                                    }),
                                )
                                .child(icon(ICON_COPY, ICON_MD, theme.colors.text_muted))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_primary)
                                        .child("Duplicate"),
                                ),
                        )
                    })
                    .child(
                        div()
                            .id("context-menu-rename")
                            .w_full()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .gap(px(8.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, window, cx| {
                                    this.start_rename_item(path_for_rename.clone(), cx);
                                    this.edit_focus.focus(window, cx);
                                }),
                            )
                            .child(icon(ICON_EDIT, ICON_MD, theme.colors.text_muted))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_primary)
                                    .child("Rename"),
                            ),
                    )
                    .child(
                        div()
                            .id("context-menu-delete")
                            .w_full()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .gap(px(8.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.status_client_error.opacity(0.1)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.delete_collection_item(path_for_delete.clone(), cx);
                                }),
                            )
                            .child(icon(ICON_DELETE, ICON_MD, theme.colors.status_client_error))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.status_client_error)
                                    .child(if is_folder { "Delete Folder" } else { "Delete" }),
                            ),
                    ),
            )
    }
}
