use super::*;
use gpui::{Context, IntoElement, MouseDownEvent, ParentElement, SharedString, Styled, div, px};

/// Method badge width: fits the longest method, OPTIONS, at the badge font.
const BADGE_W: f32 = 46.0;

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
        let is_expandable = item.is_expandable();
        let is_expanded = item.expanded;
        let request_index = item.request_index;
        let is_request_row = request_index.is_some();
        let display_name = if is_request_row {
            item.name.clone()
        } else {
            item.name.trim_end_matches(".http").to_string()
        };
        let method = item.method.clone();
        let has_badge = method.is_some();
        let is_selected = self.selected_item.as_ref() == Some(&item.path)
            && self.selected_request == request_index;
        let is_renaming = !is_request_row && self.renaming_item.as_ref() == Some(&item.path);
        let is_dirty = self
            .request_panel
            .as_ref()
            .and_then(|rp| rp.read(cx).dirty_location())
            .is_some_and(|(file, i)| file == item.path && request_index.is_none_or(|r| r == i));
        let has_watcher = self.workspace_watcher.is_some();
        let guide_color = theme.colors.text_muted.opacity(0.12);

        let mut row = ActionRow::new(
            SharedString::from(format!("collection-item-{}", idx)),
            SharedString::from(format!("coll-row-{}", idx)),
            &theme,
        )
        .selected(is_selected)
        .on_click(cx.listener(move |this, _, _, cx| {
            if is_expandable {
                this.selected_item = Some(path_for_select.clone());
                this.selected_request = request_index;
                this.toggle_collection_folder(path.clone(), cx);
            } else {
                this.open_request(path.clone(), request_index, cx);
            }
        }))
        .on_right_click(cx.listener(move |this, event: &MouseDownEvent, _, cx| {
            this.selected_item = Some(path_for_right_click.clone());
            this.selected_request = request_index;
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

        // `min_w(0)` lets the row shrink below its text so the name can
        // truncate instead of running past the sidebar edge.
        let content = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .pl(indent)
            .pr(px(4.0))
            .when(is_expandable, |el| {
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
            .when(!is_expandable, |el| el.child(div().w(px(10.0))))
            .child(div().w(px(crate::theme::sizes::CHEVRON_ICON_GAP)))
            .when(!is_request_row, |el| {
                el.child(if is_folder {
                    icon(ICON_FOLDER, ICON_MD, theme.colors.text_muted)
                } else {
                    icon(ICON_FILE, ICON_MD, theme.colors.accent)
                })
                .child(div().w(px(crate::theme::sizes::ICON_TEXT_GAP)))
            })
            .when_some(method, |el, method| {
                let color = theme.method_color(&method);
                el.child(
                    // Fixed width so names line up whatever the method.
                    div()
                        .flex_none()
                        .w(px(BADGE_W))
                        .h(px(16.0))
                        .overflow_hidden()
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
                        .truncate()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_primary)
                        .when(is_folder, |el| el.font_weight(gpui::FontWeight::SEMIBOLD))
                        .child(display_name),
                )
            })
            .when(is_dirty, |el| {
                el.child(
                    div()
                        .id(SharedString::from(format!("coll-dirty-{}", idx)))
                        .pl(px(4.0))
                        .text_size(px(10.0))
                        .text_color(theme.colors.warning)
                        .tooltip(tooltip_text("Unsaved changes"))
                        .child("●"),
                )
            })
            .when(is_renaming, |el| el.child(self.render_rename_input(cx)));

        row = row.child(content);

        // Revealed on hover like the "+" action, not shown permanently.
        if is_folder && has_watcher {
            row = row.action(
                div()
                    .id(SharedString::from(format!("watched-{}", idx)))
                    .px(px(4.0))
                    .tooltip(tooltip_text("Watched for changes"))
                    .child(icon(ICON_LINK, ICON_SM, theme.colors.team_accent)),
            );
        }

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
