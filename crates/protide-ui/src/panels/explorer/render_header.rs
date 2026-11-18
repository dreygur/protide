use super::*;
use gpui::{Context, IntoElement, ParentElement, Styled, div, px};

impl ExplorerPanel {
    pub(super) fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(14.0))
            .border_b_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .id("toggle-sidebar-btn")
                            .size(px(22.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_click({
                                let main_window = self.main_window.clone();
                                cx.listener(move |_this, _, _, cx| {
                                    if let Some(win) = main_window.upgrade() {
                                        win.update(cx, |w, cx| w.toggle_sidebar(cx));
                                    }
                                })
                            })
                            .child(icon(ICON_MENU, ICON_MD, theme.colors.text_muted)),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Explorer"),
                    ),
            )
            .child(
                div()
                    .id("new-request-btn")
                    .size(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_size(px(14.0))
                    .text_color(theme.colors.text_muted)
                    .hover(|s| {
                        s.bg(theme.colors.bg_tertiary)
                            .text_color(theme.colors.text_primary)
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.create_new_request(cx);
                    }))
                    .child("+"),
            )
    }
}
