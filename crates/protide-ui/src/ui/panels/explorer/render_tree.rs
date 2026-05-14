use gpui::{Context, IntoElement, ParentElement, ScrollWheelEvent, Styled, div, px};
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
                                icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                            } else {
                                icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                            })
                            .child(div().w(px(crate::theme::sizes::CHEVRON_ICON_GAP)))
                            .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_muted))
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
        let has_collections = !self.collection_items.is_empty();
        div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .child(
                icon_btn("refresh-collections-btn", ICON_REFRESH, cx)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.refresh_collections(cx);
                    })),
            )
            .when(has_collections, |el| {
                el.child(
                    icon_btn("collapse-all-btn", ICON_CHEVRON_UP, cx)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.collapse_all_folders(cx);
                        })),
                )
            })
            .child(
                icon_btn("open-folder-btn", ICON_FOLDER_OPEN, cx)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.open_folder(cx);
                    })),
            )
            .child(
                icon_btn("import-collection-btn", ICON_ARROW_DOWN, cx)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.import_collection(cx);
                    })),
            )
            .when(self.workspace_path.is_some(), |el| {
                el.child(
                    icon_btn("export-docs-btn", ICON_EXTERNAL, cx)
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
                        .text_color(theme.colors.text_muted)
                        .hover(|s| {
                            s.bg(theme.colors.bg_tertiary)
                                .text_color(theme.colors.status_client_error)
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close_project(cx);
                        }))
                        .child(icon(ICON_CLOSE, ICON_MD, theme.colors.text_muted)),
                )
            })
    }

    fn render_collections_tree(
        &mut self,
        el: gpui::Div,
        flattened_items: Vec<(CollectionItem, usize)>,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        const ROW_H: f32 = 28.0;
        let viewport_h = (self.collections_h - 44.0).max(0.0);
        let indexed: Vec<(usize, CollectionItem, usize)> = flattened_items
            .into_iter()
            .enumerate()
            .map(|(i, (item, depth))| (i, item, depth))
            .collect();
        let total_items = indexed.len();
        let max_scroll = (total_items as f32 * ROW_H - viewport_h).max(0.0);
        let scroll = self.tree_scroll.min(max_scroll);
        let start_idx = (scroll / ROW_H).floor() as usize;
        let visible_count = (viewport_h / ROW_H).ceil() as usize + 2;
        let end_idx = (start_idx + visible_count).min(total_items);
        let top_h = start_idx as f32 * ROW_H;
        let bot_h = (total_items - end_idx) as f32 * ROW_H;

        el.child(
            div()
                .id("collections-tree")
                .w_full()
                .flex()
                .flex_col()
                .px(px(4.0))
                .pt(px(4.0))
                .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
                    let b = this.panel_bounds;
                    let ey = f32::from(event.position.y);
                    let col_top = f32::from(b.origin.y) + 40.0;
                    let col_bot = col_top + this.collections_h;
                    if ey < col_top || ey > col_bot { return; }
                    let ex = f32::from(event.position.x);
                    if ex < f32::from(b.origin.x) || ex > f32::from(b.origin.x) + f32::from(b.size.width) { return; }
                    let vp = (this.collections_h - 44.0).max(0.0);
                    let n = Self::count_visible_items(&this.collection_items);
                    let max = (n as f32 * ROW_H - vp).max(0.0);
                    let delta = f32::from(event.delta.pixel_delta(px(ROW_H)).y);
                    this.tree_scroll = (this.tree_scroll - delta).clamp(0.0, max);
                    cx.notify();
                }))
                .when(top_h > 0.0, |el| el.child(div().w_full().h(px(top_h))))
                .children(
                    indexed[start_idx..end_idx]
                        .iter()
                        .map(|(idx, item, depth)| {
                            self.render_collection_item_row(item.clone(), *depth, *idx, cx)
                        }),
                )
                .when(bot_h > 0.0, |el| el.child(div().w_full().h(px(bot_h)))),
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
