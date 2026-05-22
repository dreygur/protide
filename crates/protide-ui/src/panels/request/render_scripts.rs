//! Scripts tab rendering for RequestPanel


use gpui::{
    div, prelude::*, px, Context, IntoElement, MouseDownEvent, MouseMoveEvent,
    ParentElement, Styled,
};

use crate::theme;
use crate::components::icons::{
    icon, ICON_SM,
    ICON_CHECK, ICON_CHEVRON_DOWN, ICON_CHEVRON_RIGHT, ICON_CHEVRON_LEFT,
};
use protide_core::execution::ws::WebSocketExecutor;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_scripts_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let script_pre_open = self.script_pre_open;
        let script_post_open = self.script_post_open;
        let script_tests_open = self.script_tests_open;
        let script_pre_h = self.script_pre_h;
        let script_post_h = self.script_post_h;

        div()
            .id("scripts-tab")
            .w_full()
            .h_full()
            .relative()
            .flex()
            .flex_col()
            // Pre-request Script section
            .child(
                div()
                    .id("script-pre-header")
                    .h(px(36.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.5)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.script_pre_open = !this.script_pre_open;
                        cx.notify();
                    }))
                    .child(if script_pre_open {
                        icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                    } else {
                        icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                    })
                    .child(
                        div()
                            .size(px(18.0))
                            .bg(theme.colors.method_post.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.method_post))
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Pre-request Script")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Runs before sending the request")
                    )
            )
            .when(script_pre_open, |el| {
                el.child(
                    div()
                        .h(px(script_pre_h))
                        .w_full()
                        .overflow_hidden()
                        .child(gpui_component::input::Input::new(&self.pre_script_editor)),
                )
                .child(
                    div()
                        .id("drag-script-pre")
                        .w_full()
                        .h(px(4.0))
                        .cursor_row_resize()
                        .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, event: &MouseDownEvent, _, _| {
                                this.drag_script_pre =
                                    Some((f32::from(event.position.y), this.script_pre_h));
                            }),
                        ),
                )
            })
            // Post-response Script section
            .child(
                div()
                    .id("script-post-header")
                    .h(px(36.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.5)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.script_post_open = !this.script_post_open;
                        cx.notify();
                    }))
                    .child(if script_post_open {
                        icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                    } else {
                        icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                    })
                    .child(
                        div()
                            .size(px(18.0))
                            .bg(theme.colors.status_success.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(ICON_CHEVRON_LEFT, ICON_SM, theme.colors.status_success))
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Post-response Script")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Runs after receiving response")
                    )
            )
            .when(script_post_open, |el| {
                el.child(
                    div()
                        .h(px(script_post_h))
                        .w_full()
                        .overflow_hidden()
                        .child(gpui_component::input::Input::new(&self.post_script_editor)),
                )
                .child(
                    div()
                        .id("drag-script-post")
                        .w_full()
                        .h(px(4.0))
                        .cursor_row_resize()
                        .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, event: &MouseDownEvent, _, _| {
                                this.drag_script_post =
                                    Some((f32::from(event.position.y), this.script_post_h));
                            }),
                        ),
                )
            })
            // Tests section
            .child(
                div()
                    .id("script-tests-header")
                    .h(px(36.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.5)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.script_tests_open = !this.script_tests_open;
                        cx.notify();
                    }))
                    .child(if script_tests_open {
                        icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                    } else {
                        icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                    })
                    .child(
                        div()
                            .size(px(18.0))
                            .bg(theme.colors.accent.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(ICON_CHECK, ICON_SM, theme.colors.accent))
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Tests")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Test assertions using expect()")
                    )
            )
            .when(script_tests_open, |el| {
                el.child(
                    div()
                        .flex_1()
                        .w_full()
                        .overflow_hidden()
                        .child(gpui_component::input::Input::new(&self.tests_editor)),
                )
            })
            // Pre-script drag overlay
            .when(self.drag_script_pre.is_some(), |el| {
                el.child(gpui::deferred(
                    div()
                        .id("drag-script-pre-overlay")
                        .absolute()
                        .inset_0()
                        .cursor_row_resize()
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                            if let Some((start_y, start_h)) = this.drag_script_pre {
                                let delta = f32::from(event.position.y) - start_y;
                                this.script_pre_h = (start_h + delta).max(60.0).min(600.0);
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.drag_script_pre = None;
                                crate::prefs::set_f32("request.script_pre_h", this.script_pre_h);
                                cx.notify();
                            }),
                        ),
                ).with_priority(2))
            })
            // Post-script drag overlay
            .when(self.drag_script_post.is_some(), |el| {
                el.child(gpui::deferred(
                    div()
                        .id("drag-script-post-overlay")
                        .absolute()
                        .inset_0()
                        .cursor_row_resize()
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                            if let Some((start_y, start_h)) = this.drag_script_post {
                                let delta = f32::from(event.position.y) - start_y;
                                this.script_post_h = (start_h + delta).max(60.0).min(600.0);
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.drag_script_post = None;
                                crate::prefs::set_f32("request.script_post_h", this.script_post_h);
                                cx.notify();
                            }),
                        ),
                ).with_priority(2))
            })
            .into_any_element()
    }
}
