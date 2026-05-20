use gpui::{Context, IntoElement, ParentElement, Styled, div, px, white};
use super::*;

impl ExplorerPanel {
    pub(super) fn render_collections_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let has_collections = !self.collection_items.is_empty();
        let collections_expanded = self.collections_expanded;
        let workspace_name = self
            .workspace_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Collections".to_string());

        let flattened_items = if has_collections && collections_expanded {
            self.flatten_collection_items(&self.collection_items, 0)
        } else {
            Vec::new()
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(8.0))
            .child(
                div()
                    .id("collections-header")
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .mx(px(4.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_collections(cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(if collections_expanded {
                                icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_secondary)
                            } else {
                                icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_secondary)
                            })
                            .child(div().w(px(crate::theme::sizes::CHEVRON_ICON_GAP)))
                            .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_secondary))
                            .child(div().w(px(crate::theme::sizes::ICON_TEXT_GAP)))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child(workspace_name),
                            ),
                    )
                    .child(self.render_collections_header_buttons(cx)),
            )
            .when(collections_expanded, |el| {
                if has_collections {
                    self.render_collections_tree(el, flattened_items, cx)
                } else {
                    el.child(self.render_collections_empty(cx))
                }
            })
    }

    fn render_collections_header_buttons(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let has_workspace = self.workspace_path.is_some();
        let has_collections = !self.collection_items.is_empty();
        div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .when(has_workspace, |el| {
                el.child(
                    icon_btn("refresh-collections-btn", ICON_REFRESH, cx)
                        .tooltip(tooltip_text("Refresh collections"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.refresh_collections(cx);
                        })),
                )
                .when(has_collections, |el| {
                    el.child(
                        icon_btn("collapse-all-btn", ICON_CHEVRON_UP, cx)
                            .tooltip(tooltip_text("Collapse all folders"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.collapse_all_folders(cx);
                            })),
                    )
                })
            })
            .child(
                icon_btn("open-folder-btn", ICON_FOLDER_OPEN, cx)
                    .tooltip(tooltip_text("Open folder"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.open_folder(cx);
                    })),
            )
            .child(
                icon_btn("import-collection-btn", ICON_ARROW_DOWN, cx)
                    .tooltip(tooltip_text("Import collection"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.import_collection(cx);
                    })),
            )
            .when(has_workspace, |el| {
                el.child(
                    icon_btn("export-docs-btn", ICON_EXPORT, cx)
                        .tooltip(tooltip_text("Export API docs"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.export_docs(cx);
                        })),
                )
                .child(
                    div()
                        .id("close-project-btn")
                        .size(px(22.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                        .tooltip(tooltip_text("Close project"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close_project(cx);
                        }))
                        .child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .opacity(0.45)
                                .hover(|s| s.opacity(1.0).text_color(theme.colors.status_client_error))
                                .child(icon(ICON_CLOSE, ICON_MD, white()))
                        ),
                )
            })
    }

    fn render_collections_tree(
        &mut self,
        el: gpui::Div,
        flattened_items: Vec<(CollectionItem, usize)>,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        const HEADER_H: f32 = 44.0; // pt(8) + h(32) + pt(4)
        let tree_h = (self.collections_h - HEADER_H).max(0.0);

        el.child(
            div()
                .id("collections-tree")
                .w_full()
                .h(px(tree_h))
                .overflow_scroll()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .px(px(4.0))
                        .pt(px(4.0))
                        .children(
                            flattened_items
                                .into_iter()
                                .enumerate()
                                .map(|(idx, (item, depth))| {
                                    self.render_collection_item_row(item, depth, idx, cx)
                                }),
                        ),
                ),
        )
    }

    fn render_collections_empty(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        div()
            .w_full()
            .px(px(16.0))
            .py(px(20.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .size(px(40.0))
                    .bg(theme.colors.bg_tertiary)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_muted)),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.colors.text_secondary)
                    .child("No collections"),
            )
            .child(
                div()
                    .id("open-folder-empty-btn")
                    .mt(px(4.0))
                    .px(px(12.0))
                    .py(px(6.0))
                    .cursor_pointer()
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.colors.accent)
                    .border_1()
                    .border_color(theme.colors.accent.opacity(0.3))
                    .hover(|s| s.bg(theme.colors.accent.opacity(0.1)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.open_folder(cx);
                    }))
                    .child("Open Folder"),
            )
    }
}
