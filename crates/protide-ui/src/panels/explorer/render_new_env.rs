use super::*;
use gpui::{Context, IntoElement, ParentElement, Styled, div, px};

impl ExplorerPanel {
    pub(super) fn render_new_env_form(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let new_env_name = self.new_env_name.clone();
        let is_editing_new_env = self.active_edit == Some(EnvEditTarget::NewEnvName);
        let edit_selection = self.edit_selection.clone();

        div()
            .w_full()
            .p(px(10.0))
            .bg(theme.colors.bg_primary)
            .border_1()
            .border_color(theme.colors.accent.opacity(0.3))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.colors.text_secondary)
                    .child("Create New Environment"),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(self.render_text_input(
                        "new-env-name",
                        EnvEditTarget::NewEnvName,
                        &new_env_name,
                        "Environment name",
                        is_editing_new_env,
                        if is_editing_new_env {
                            edit_selection.clone()
                        } else {
                            0..0
                        },
                        cx,
                    ))
                    .child(
                        div()
                            .id("create-env-btn")
                            .h(px(28.0))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .bg(theme.colors.accent)
                            .text_color(theme.colors.bg_primary)
                            .hover(|s| s.bg(theme.colors.accent.opacity(0.9)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.create_new_env(cx);
                            }))
                            .child("Create"),
                    )
                    .child(
                        div()
                            .id("cancel-env-btn")
                            .h(px(28.0))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_secondary)
                            .border_1()
                            .border_color(theme.colors.border)
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel_new_env(cx);
                            }))
                            .child("Cancel"),
                    ),
            )
    }
}
