//! Scripts tab rendering for RequestPanel

use crate::components::code_input;
use gpui::{
    App, Context, Div, Hsla, IntoElement, MouseDownEvent, MouseMoveEvent, ParentElement, Stateful,
    Styled, div, prelude::*, px,
};

use super::RequestPanel;
use crate::components::icons::{
    ICON_CHECK, ICON_CHEVRON_DOWN, ICON_CHEVRON_LEFT, ICON_CHEVRON_RIGHT, ICON_SM, icon,
};
use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;

/// Smallest editor height that still shows a few lines when every section is open.
const SCRIPT_MIN_H: f32 = 120.0;

/// Collapsible header shared by the three script sections. Caller adds .on_click().
fn script_header(
    id: &'static str,
    open: bool,
    (badge_icon, badge_color): (&'static str, Hsla),
    title: &'static str,
    hint: &'static str,
    cx: &App,
) -> Stateful<Div> {
    let theme = theme::current(cx);
    div()
        .id(id)
        .h(px(36.0))
        .flex_shrink_0()
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(12.0))
        .border_b_1()
        .border_color(theme.colors.border.opacity(0.5))
        .cursor_pointer()
        .hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.5)))
        .child(icon(
            if open {
                ICON_CHEVRON_DOWN
            } else {
                ICON_CHEVRON_RIGHT
            },
            ICON_SM,
            theme.colors.text_muted,
        ))
        .child(
            div()
                .size(px(18.0))
                .bg(badge_color.opacity(0.15))
                .flex()
                .items_center()
                .justify_center()
                .child(icon(badge_icon, ICON_SM, badge_color)),
        )
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.colors.text_primary)
                .child(title),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(theme.colors.text_muted)
                .child(hint),
        )
}

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_scripts_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let script_pre_open = self.script_pre_open;
        let script_post_open = self.script_post_open;
        let script_tests_open = self.script_tests_open;
        let script_pre_h = self.script_pre_h;
        let script_post_h = self.script_post_h;

        let open_count =
            script_pre_open as usize + script_post_open as usize + script_tests_open as usize;
        let single_open = open_count == 1;

        div()
            .id("scripts-tab")
            .w_full()
            .min_h_full()
            .relative()
            .flex()
            .flex_col()
            // Pre-request Script section
            .child(
                script_header(
                    "script-pre-header",
                    script_pre_open,
                    (ICON_CHEVRON_RIGHT, theme.colors.method_post),
                    "Pre-request Script",
                    "Runs before sending the request",
                    cx,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.script_pre_open = !this.script_pre_open;
                    cx.notify();
                })),
            )
            .when(script_pre_open, |el| {
                let editor_div = if single_open {
                    div()
                        .flex_1()
                        .min_h(px(SCRIPT_MIN_H))
                        .w_full()
                        .overflow_hidden()
                        .child(
                            code_input(&self.pre_script_editor)
                                .appearance(false)
                                .h_full(),
                        )
                } else {
                    div()
                        .h(px(script_pre_h))
                        .flex_shrink_0()
                        .w_full()
                        .overflow_hidden()
                        .child(
                            code_input(&self.pre_script_editor)
                                .appearance(false)
                                .h_full(),
                        )
                };
                let el = el.child(editor_div);
                if !single_open {
                    el.child(
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
                } else {
                    el
                }
            })
            // Post-response Script section
            .child(
                script_header(
                    "script-post-header",
                    script_post_open,
                    (ICON_CHEVRON_LEFT, theme.colors.status_success),
                    "Post-response Script",
                    "Runs after receiving response",
                    cx,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.script_post_open = !this.script_post_open;
                    cx.notify();
                })),
            )
            .when(script_post_open, |el| {
                let editor_div = if single_open {
                    div()
                        .flex_1()
                        .min_h(px(SCRIPT_MIN_H))
                        .w_full()
                        .overflow_hidden()
                        .child(
                            code_input(&self.post_script_editor)
                                .appearance(false)
                                .h_full(),
                        )
                } else {
                    div()
                        .h(px(script_post_h))
                        .flex_shrink_0()
                        .w_full()
                        .overflow_hidden()
                        .child(
                            code_input(&self.post_script_editor)
                                .appearance(false)
                                .h_full(),
                        )
                };
                let el = el.child(editor_div);
                if !single_open {
                    el.child(
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
                } else {
                    el
                }
            })
            // Tests section
            .child(
                script_header(
                    "script-tests-header",
                    script_tests_open,
                    (ICON_CHECK, theme.colors.accent),
                    "Tests",
                    "Test assertions using expect()",
                    cx,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.script_tests_open = !this.script_tests_open;
                    cx.notify();
                })),
            )
            .when(script_tests_open, |el| {
                el.child(
                    div()
                        .flex_1()
                        .min_h(px(SCRIPT_MIN_H))
                        .w_full()
                        .overflow_hidden()
                        .child(code_input(&self.tests_editor).appearance(false).h_full()),
                )
            })
            // Pre-script drag overlay (only when multi-open and dragging)
            .when(self.drag_script_pre.is_some(), |el| {
                el.child(
                    gpui::deferred(
                        div()
                            .id("drag-script-pre-overlay")
                            .absolute()
                            .inset_0()
                            .cursor_row_resize()
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                                if let Some((start_y, start_h)) = this.drag_script_pre {
                                    let delta = f32::from(event.position.y) - start_y;
                                    this.script_pre_h = (start_h + delta).clamp(60.0, 600.0);
                                    cx.notify();
                                }
                            }))
                            .on_mouse_up(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.drag_script_pre = None;
                                    crate::prefs::set_f32(
                                        "request.script_pre_h",
                                        this.script_pre_h,
                                    );
                                    cx.notify();
                                }),
                            ),
                    )
                    .with_priority(2),
                )
            })
            // Post-script drag overlay (only when multi-open and dragging)
            .when(self.drag_script_post.is_some(), |el| {
                el.child(
                    gpui::deferred(
                        div()
                            .id("drag-script-post-overlay")
                            .absolute()
                            .inset_0()
                            .cursor_row_resize()
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                                if let Some((start_y, start_h)) = this.drag_script_post {
                                    let delta = f32::from(event.position.y) - start_y;
                                    this.script_post_h = (start_h + delta).clamp(60.0, 600.0);
                                    cx.notify();
                                }
                            }))
                            .on_mouse_up(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.drag_script_post = None;
                                    crate::prefs::set_f32(
                                        "request.script_post_h",
                                        this.script_post_h,
                                    );
                                    cx.notify();
                                }),
                            ),
                    )
                    .with_priority(2),
                )
            })
            .into_any_element()
    }
}
