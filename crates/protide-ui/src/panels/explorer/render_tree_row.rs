use gpui::{Context, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, SharedString, Styled, Window, div, px};
use super::*;

impl ExplorerPanel {
    /// Render a single collection item row (non-recursive)
    pub(super) fn render_collection_item_row(
        &self,
        item: CollectionItem,
        depth: usize,
        idx: usize,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let indent = px((depth * 16 + 8) as f32);
        let path = item.path.clone();
        let path_for_right_click = item.path.clone();
        let path_for_select = item.path.clone();
        let path_for_action = item.path.clone();
        let is_folder = item.is_folder;
        let is_expanded = item.expanded;
        let display_name = item.name.trim_end_matches(".http").to_string();
        let method = item.method.clone();
        let has_badge = method.is_some();
        let is_selected = self.selected_item.as_ref() == Some(&item.path);
        let is_renaming = self.renaming_item.as_ref() == Some(&item.path);
        let has_watcher = self.workspace_watcher.is_some();
        let guide_color = theme.colors.text_muted.opacity(0.12);

        let mut row = ActionRow::new(
            SharedString::from(format!("collection-item-{}", idx)),
            SharedString::from(format!("coll-row-{}", idx)),
            &theme,
        )
        .selected(is_selected)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.selected_item = Some(path_for_select.clone());
            if is_folder {
                this.toggle_collection_folder(path.clone(), cx);
            } else {
                this.load_request_file(path.clone(), cx);
            }
        }))
        .on_right_click(cx.listener(move |this, event: &MouseDownEvent, _, cx| {
            this.selected_item = Some(path_for_right_click.clone());
            this.context_menu = Some((path_for_right_click.clone(), event.position));
            cx.notify();
        }));

        for level in 0..depth {
            row = row.child(
                div()
                    .absolute()
                    .left(px(8.0 + level as f32 * 16.0 + 7.5))
                    .top(px(0.0))
                    .h(px(28.0))
                    .w(px(1.0))
                    .bg(guide_color),
            );
        }

        let rename_text = self.rename_text.clone();
        let content = div()
            .flex_1()
            .flex()
            .items_center()
            .pl(indent)
            .pr(px(4.0))
            .when(is_folder, |el| {
                el.child(
                    div()
                        .w(px(10.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if is_expanded {
                            icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                        } else {
                            icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                        }),
                )
            })
            .when(!is_folder, |el| el.child(div().w(px(10.0))))
            .child(div().w(px(crate::theme::sizes::CHEVRON_ICON_GAP)))
            .child(if is_folder {
                icon(ICON_FOLDER, ICON_MD, theme.colors.text_muted)
            } else {
                icon(ICON_FILE, ICON_MD, theme.colors.accent)
            })
            .child(div().w(px(crate::theme::sizes::ICON_TEXT_GAP)))
            .when_some(method, |el, method| {
                let color = theme.method_color(&method);
                el.child(
                    div()
                        .min_w(px(36.0))
                        .h(px(16.0))
                        .px(px(4.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(color.opacity(0.12))
                        .border_1()
                        .border_color(color.opacity(0.6))
                        .child(
                            div()
                                .text_size(px(9.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .font_family("JetBrains Mono")
                                .text_color(color)
                                .child(method),
                        ),
                )
            })
            .when(has_badge, |el| el.child(div().w(px(4.0))))
            .when(!is_renaming, |el| {
                el.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_primary)
                        .when(is_folder, |el| el.font_weight(gpui::FontWeight::SEMIBOLD))
                        .child(display_name),
                )
            })
            .when(is_renaming, |el| {
                el.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .h(px(28.0))
                        .px(px(4.0))
                        .flex()
                        .items_center()
                        .gap(px(0.0))
                        .bg(theme.colors.bg_tertiary)
                        .border_1()
                        .border_color(theme.colors.accent)
                        .overflow_hidden()
                        .cursor_text()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window: &mut Window, cx| {
                            cx.stop_propagation();
                            this.edit_focus.focus(window, cx);
                        }))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_primary)
                                .child(rename_text)
                        )
                        .child(
                            div()
                                .w(px(1.5))
                                .h(px(14.0))
                                .bg(theme.colors.accent)
                        ),
                )
            })
            .when(is_folder && has_watcher, |el| {
                el.child(div().w(px(4.0)))
                    .child(icon(ICON_LINK, ICON_SM, theme.colors.team_accent))
            });

        row = row.child(content);

        if is_folder {
            row = row.action(
                ghost_action_btn(
                    SharedString::from(format!("new-in-folder-{}", idx)),
                    ICON_PLUS,
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.create_new_file_in_folder(path_for_action.clone(), cx);
                })),
            );
        }

        row
    }

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
        let left = if x + MENU_W > panel_w { px((x - MENU_W).max(0.0)) } else { px(x) };
        let top  = if y + menu_h > panel_h { px((y - menu_h).max(0.0)) } else { px(y) };

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
            .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                cx.stop_propagation();
            }))
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
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                    this.run_collection_folder(path_for_run.clone(), cx);
                                }))
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
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                    this.export_openapi_folder(path_for_export.clone(), cx);
                                }))
                                .child(icon(ICON_EXTERNAL, ICON_MD, theme.colors.accent))
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
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                    this.context_menu = None;
                                    this.clone_file(path_for_clone.clone(), cx);
                                }))
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
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                                this.start_rename_item(path_for_rename.clone(), cx);
                                this.edit_focus.focus(window, cx);
                            }))
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
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                this.delete_collection_item(path_for_delete.clone(), cx);
                            }))
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
