use gpui::{Context, IntoElement, MouseButton, ParentElement, SharedString, Styled, div, px};
use super::*;

impl ExplorerPanel {
    pub(super) fn render_environment_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let active_env_name = self
            .env_state
            .active()
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "No Environment".to_string());
        let env_dropdown_open = self.env_dropdown_open;
        let env_editor_open = self.env_editor_open;
        let has_active_env = self.env_state.active_index.is_some();
        let var_count = self
            .env_state
            .active()
            .map(|e| e.variables.len())
            .unwrap_or(0);

        div()
            .w_full()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_tertiary.opacity(0.3))
            .child(
                div()
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(icon(ICON_SETTINGS, ICON_MD, theme.colors.text_muted))
                            .child(div().w(px(crate::theme::sizes::ICON_TEXT_GAP)))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("ENVIRONMENT"),
                            ),
                    )
                    .when(var_count > 0, |el| {
                        el.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .bg(theme.colors.status_success.opacity(0.12))
                                .text_size(px(9.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.status_success)
                                .child(format!("{} vars", var_count)),
                        )
                    }),
            )
            .child(
                div()
                    .h(px(40.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .child(
                        div()
                            .id("env-selector")
                            .flex_1()
                            .h(px(32.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(10.0))
                            .cursor_pointer()
                            .bg(theme.colors.bg_primary)
                            .border_1()
                            .border_color(if env_dropdown_open {
                                theme.colors.accent
                            } else {
                                theme.colors.border
                            })
                            .hover(|s| s.border_color(theme.colors.text_muted))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_env_dropdown(cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(div().size(px(6.0)).bg(if has_active_env {
                                        theme.colors.status_success
                                    } else {
                                        theme.colors.text_muted.opacity(0.5)
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(if has_active_env {
                                                theme.colors.text_primary
                                            } else {
                                                theme.colors.text_muted
                                            })
                                            .child(active_env_name.clone()),
                                    ),
                            )
                            .child(if env_dropdown_open {
                                icon(ICON_CHEVRON_UP, ICON_SM, theme.colors.text_muted)
                            } else {
                                icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                            }),
                    )
                    .child(
                        div()
                            .id("env-edit-btn")
                            .ml(px(8.0))
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(12.0))
                            .when(env_editor_open, |el| {
                                el.bg(theme.colors.accent.opacity(0.12))
                                    .text_color(theme.colors.accent)
                            })
                            .when(!env_editor_open, |el| {
                                el.text_color(theme.colors.text_muted).hover(|s| {
                                    s.bg(theme.colors.bg_tertiary)
                                        .text_color(theme.colors.text_primary)
                                })
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_env_editor(cx);
                            }))
                            .child(if env_editor_open {
                                icon(ICON_EDIT, ICON_MD, theme.colors.accent)
                            } else {
                                icon(ICON_EDIT, ICON_MD, theme.colors.text_muted)
                            }),
                    ),
            )
            .when(env_dropdown_open, |el| {
                el.child(self.render_env_dropdown(cx))
            })
            .when(env_editor_open, |el| el.child(self.render_env_editor(cx)))
    }

    pub(super) fn render_env_dropdown(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let active_index = self.env_state.active_index;
        let envs: Vec<(usize, String, usize)> = self
            .env_state
            .environments
            .iter()
            .enumerate()
            .map(|(i, e)| (i, e.name.clone(), e.variables.len()))
            .collect();

        div()
            .id("env-dropdown")
            .max_h(px(200.0))
            .overflow_scroll()
            .mx(px(12.0))
            .mb(px(8.0))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_primary)
            .flex()
            .flex_col()
            .child(
                div()
                    .id("env-option-none")
                    .w_full()
                    .px(px(12.0))
                    .py(px(8.0))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .when(active_index.is_none(), |el| {
                        el.bg(theme.colors.accent.opacity(0.08))
                    })
                    .when(active_index.is_some(), |el| {
                        el.hover(|s| s.bg(theme.colors.bg_tertiary))
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.select_environment(None, cx);
                    }))
                    .child(div().size(px(6.0)).bg(theme.colors.text_muted.opacity(0.4)))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child("No Environment"),
                    ),
            )
            .children(
                envs.into_iter()
                    .enumerate()
                    .map(|(idx, (i, name, var_count))| {
                        let is_active = active_index == Some(i);
                        let is_last = idx == self.env_state.environments.len() - 1;
                        div()
                            .id(SharedString::from(format!("env-option-{}", i)))
                            .w_full()
                            .px(px(12.0))
                            .py(px(8.0))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .when(!is_last, |el| {
                                el.border_b_1()
                                    .border_color(theme.colors.border.opacity(0.5))
                            })
                            .when(is_active, |el| el.bg(theme.colors.accent.opacity(0.08)))
                            .when(!is_active, |el| {
                                el.hover(|s| s.bg(theme.colors.bg_tertiary))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_environment(Some(i), cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(div().size(px(6.0)).bg(theme.colors.status_success))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(if is_active {
                                                gpui::FontWeight::MEDIUM
                                            } else {
                                                gpui::FontWeight::NORMAL
                                            })
                                            .text_color(if is_active {
                                                theme.colors.text_primary
                                            } else {
                                                theme.colors.text_secondary
                                            })
                                            .child(name),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .when(var_count > 0, |el| {
                                        el.child(
                                            div()
                                                .text_size(px(10.0))
                                                .text_color(theme.colors.text_muted)
                                                .child(format!("{} vars", var_count)),
                                        )
                                    })
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("env-edit-{}", i)))
                                            .size(px(18.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .opacity(0.4)
                                            .hover(|s| s.opacity(1.0))
                                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                this.open_env_editor_for(i, cx);
                                            }))
                                            .child(icon(ICON_EDIT, ICON_SM, theme.colors.text_secondary))
                                            .tooltip(tooltip_text("Edit environment")),
                                    ),
                            )
                    }),
            )
            .child(
                div()
                    .id("env-add-btn")
                    .w_full()
                    .px(px(12.0))
                    .py(px(8.0))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_tertiary.opacity(0.3))
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.env_dropdown_open = false;
                        this.env_editor_open = true;
                        this.start_new_env(cx);
                    }))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.accent)
                            .child("+"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.accent)
                            .child("New Environment"),
                    ),
            )
    }
}
