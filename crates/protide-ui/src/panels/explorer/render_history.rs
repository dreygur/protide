use gpui::{Context, IntoElement, ParentElement, SharedString, Styled, div, px};
use super::*;

impl ExplorerPanel {
    pub(super) fn render_history_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let entries = self.get_history_entries(cx);
        let has_entries = !entries.is_empty();
        let entry_count = entries.len();

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(4.0))
            .child(
                div()
                    .mx(px(12.0))
                    .h(px(1.0))
                    .bg(theme.colors.border.opacity(0.5)),
            )
            .child(
                div()
                    .id("history-header")
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .cursor_pointer()
                    .mx(px(4.0))
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_history(cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(if self.history_expanded {
                                icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_secondary)
                            } else {
                                icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_secondary)
                            })
                            .child(div().w(px(crate::theme::sizes::CHEVRON_ICON_GAP)))
                            .child(icon(ICON_TIMER, ICON_MD, theme.colors.text_secondary))
                            .child(div().w(px(crate::theme::sizes::ICON_TEXT_GAP)))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_primary)
                                    .child("History"),
                            ),
                    )
                    .when(entry_count > 0, |el| {
                        el.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .bg(theme.colors.accent.opacity(0.15))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.accent)
                                .child(format!("{}", entry_count)),
                        )
                    }),
            )
            .when(self.history_expanded, |el| {
                el.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .px(px(4.0))
                        .pt(px(4.0))
                        .when(!has_entries, |el| {
                            el.child(
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
                                            .child(icon(ICON_FILE, ICON_MD, theme.colors.text_muted)),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.text_secondary)
                                            .child("No history yet"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("Requests will appear here after sending"),
                                    ),
                            )
                        })
                        .children(entries.into_iter().enumerate().map(|(entry_idx, entry)| {
                            let entry_id = entry.id;
                            let method = entry.method.clone();
                            let display_url = entry.display_url();
                            let method_color = theme.method_color(&method);
                            let status = entry.status;
                            let status_color = status.map(|s| theme.status_color(s));

                            ActionRow::new(
                                SharedString::from(format!("history-{}", entry_id)),
                                SharedString::from(format!("hist-row-{}", entry_idx)),
                                &theme,
                            )
                            .height(px(32.0))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.load_history_item(entry_id, cx);
                            }))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .px(px(8.0))
                                    .gap(px(10.0))
                                    .child(
                                        div()
                                            .min_w(px(36.0))
                                            .h(px(16.0))
                                            .px(px(4.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .bg(method_color.opacity(0.12))
                                            .border_1()
                                            .border_color(method_color.opacity(0.6))
                                            .child(
                                                div()
                                                    .text_size(px(9.0))
                                                    .font_weight(gpui::FontWeight::BOLD)
                                                    .font_family("JetBrains Mono")
                                                    .text_color(method_color)
                                                    .child(method),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.text_primary)
                                            .child(display_url),
                                    )
                                    .when_some(status_color, |el, color| {
                                        el.child(div().size(px(7.0)).bg(color))
                                    }),
                            )
                        })),
                )
            })
    }
}
