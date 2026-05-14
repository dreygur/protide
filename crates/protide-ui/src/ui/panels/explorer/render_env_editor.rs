use gpui::{Context, IntoElement, MouseButton, MouseDownEvent, ParentElement, SharedString, Styled, div, px};
use super::*;

impl ExplorerPanel {
    pub(super) fn render_env_editor(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let show_new_env_input = self.show_new_env_input;
        let new_env_name = self.new_env_name.clone();
        let is_editing_new_env = self.active_edit == Some(EnvEditTarget::NewEnvName);
        let edit_selection = self.edit_selection.clone();

        let vars: Vec<(usize, String, String)> = self
            .env_state
            .active()
            .map(|e| {
                e.variables
                    .iter()
                    .enumerate()
                    .map(|(i, (k, v))| (i, k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let env_count = self.env_state.environments.len();
        let active_index = self.env_state.active_index;
        let has_vars = !vars.is_empty();

        div()
            .id("env-editor")
            .w_full()
            .h(px(self.env_h))
            .overflow_scroll()
            .px(px(12.0))
            .pb(px(12.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .when(show_new_env_input, |el| {
                el.child(
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
                                    if is_editing_new_env { edit_selection.clone() } else { 0..0 },
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
                        ),
                )
            })
            .when(self.env_state.active_index.is_some(), |el| {
                el.child(
                    div()
                        .w_full()
                        .bg(theme.colors.bg_primary)
                        .border_1()
                        .border_color(theme.colors.border)
                        .overflow_hidden()
                        .child(
                            div()
                                .w_full()
                                .h(px(32.0))
                                .flex()
                                .items_center()
                                .justify_between()
                                .px(px(10.0))
                                .bg(theme.colors.bg_tertiary.opacity(0.3))
                                .border_b_1()
                                .border_color(theme.colors.border.opacity(0.5))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.0))
                                        .bg(theme.colors.bg_secondary)
                                        .py(px(6.0))
                                        .child(
                                            div()
                                                .w(px(self.env_col_key_w))
                                                .text_size(px(10.0))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(theme.colors.text_muted)
                                                .child("KEY"),
                                        )
                                        .child(self.render_env_col_drag_handle(cx))
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_size(px(10.0))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(theme.colors.text_muted)
                                                .child("VALUE"),
                                        ),
                                )
                                .when(env_count > 1 && active_index.is_some(), |el| {
                                    let idx = active_index.unwrap();
                                    el.child(
                                        div()
                                            .id("delete-env-btn")
                                            .size(px(22.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .hover(|s| {
                                                s.bg(theme.colors.status_client_error.opacity(0.1))
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.delete_environment(idx, cx);
                                            }))
                                            .child(icon(
                                                ICON_DELETE,
                                                ICON_SM,
                                                theme.colors.text_muted.opacity(0.5),
                                            )),
                                    )
                                }),
                        )
                        .children(vars.into_iter().enumerate().map(|(idx, (i, key, value))| {
                            let is_editing_key = self.active_edit == Some(EnvEditTarget::VarKey(i));
                            let is_editing_value = self.active_edit == Some(EnvEditTarget::VarValue(i));
                            let key_for_remove = key.clone();

                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .px(px(10.0))
                                .py(px(6.0))
                                .when(idx % 2 == 0, |el| {
                                    el.bg(theme.colors.bg_tertiary.opacity(0.2))
                                })
                                .child(self.render_text_input_w(
                                    format!("var-key-{}", i),
                                    EnvEditTarget::VarKey(i),
                                    &key,
                                    "Key",
                                    is_editing_key,
                                    if is_editing_key { edit_selection.clone() } else { 0..0 },
                                    self.env_col_key_w,
                                    cx,
                                ))
                                .child(div().w(px(4.0)))
                                .child(self.render_text_input(
                                    format!("var-value-{}", i),
                                    EnvEditTarget::VarValue(i),
                                    &value,
                                    "Value",
                                    is_editing_value,
                                    if is_editing_value { edit_selection.clone() } else { 0..0 },
                                    cx,
                                ))
                                .child(
                                    div()
                                        .id(SharedString::from(format!("var-remove-{}", i)))
                                        .size(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_muted.opacity(0.5))
                                        .hover(|s| {
                                            s.bg(theme.colors.status_client_error.opacity(0.1))
                                                .text_color(theme.colors.status_client_error)
                                        })
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.remove_variable(&key_for_remove, cx);
                                        }))
                                        .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted.opacity(0.5))),
                                )
                        }))
                        .when(has_vars, |el| {
                            el.child(
                                div()
                                    .id("add-var-ghost")
                                    .w_full()
                                    .h(px(32.0))
                                    .flex()
                                    .items_center()
                                    .px(px(10.0))
                                    .gap(px(6.0))
                                    .cursor_pointer()
                                    .border_t_1()
                                    .border_color(theme.colors.border.opacity(0.3))
                                    .hover(|s| s.bg(theme.colors.accent.opacity(0.06)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.add_variable(cx);
                                    }))
                                    .child(icon(ICON_PLUS, ICON_SM, theme.colors.accent.opacity(0.6)))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.accent.opacity(0.6))
                                            .child("Add variable"),
                                    ),
                            )
                        })
                        .when(!has_vars, |el| {
                            el.child(
                                div()
                                    .w_full()
                                    .py(px(16.0))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(18.0))
                                            .text_color(theme.colors.text_muted.opacity(0.5))
                                            .child("{ }"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("No variables defined"),
                                    )
                                    .child(
                                        div()
                                            .id("add-first-var-btn")
                                            .mt(px(4.0))
                                            .px(px(10.0))
                                            .py(px(4.0))
                                            .cursor_pointer()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.accent)
                                            .border_1()
                                            .border_color(theme.colors.accent.opacity(0.3))
                                            .hover(|s| s.bg(theme.colors.accent.opacity(0.1)))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.add_variable(cx);
                                            }))
                                            .child("+ Add Variable"),
                                    ),
                            )
                        }),
                )
            })
            .when(
                self.env_state.active_index.is_none() && !show_new_env_input,
                |el| {
                    el.child(
                        div()
                            .w_full()
                            .py(px(16.0))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .size(px(36.0))
                                    .bg(theme.colors.bg_tertiary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icon(ICON_SETTINGS, ICON_MD, theme.colors.text_muted)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("Select an environment to edit"),
                            ),
                    )
                },
            )
            .child(
                div()
                    .w_full()
                    .mt(px(4.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .bg(theme.colors.bg_tertiary.opacity(0.5))
                    .flex()
                    .items_start()
                    .gap(px(6.0))
                    .child(icon(ICON_INFO, ICON_MD, theme.colors.text_muted))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Use {{var}} in URL, headers, body, or auth"),
                    ),
            )
    }

    pub(super) fn render_env_col_drag_handle(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let start_w = self.env_col_key_w;
        div()
            .id("env-col-drag-handle")
            .w(px(4.0))
            .self_stretch()
            .cursor_col_resize()
            .bg(theme.colors.border.opacity(0.3))
            .hover(|s| s.bg(theme.colors.accent.opacity(0.5)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.env_col_drag = Some((f32::from(event.position.x), start_w));
                    cx.notify();
                }),
            )
    }
}
