use super::*;
use gpui::{Context, IntoElement, MouseDownEvent, ParentElement, SharedString, Styled, div, px};

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
            .when(is_renaming, |el| el.child(self.render_rename_input(cx)))
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
}
