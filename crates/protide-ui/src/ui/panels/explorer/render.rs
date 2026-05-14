use gpui::{Context, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Render, Styled, Window, canvas, deferred, div, px};
use super::*;

impl Render for ExplorerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Poll file system changes on each render (cheap: just drains a channel)
        self.poll_workspace_changes(cx);
        let theme = theme::current(cx);

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_secondary)
            .track_focus(&self.edit_focus)
            .child({
                let entity = cx.entity();
                canvas(
                    move |bounds, _, cx| {
                        entity.update(cx, |this, _| {
                            this.panel_bounds = bounds;
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute().top_0().left_0().size_full()
            })
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_edit_key(event, cx);
            }))
            // Header
            .child(
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
                    ),
            )
            .child(
                div()
                    .w_full()
                    .h(px(if self.collections_expanded { self.collections_h } else { 48.0 }))
                    .overflow_hidden()
                    .child(self.render_collections_section(cx)),
            )
            .when(self.collections_expanded, |el| {
                el.child(
                    div()
                        .id("drag-handle-coll")
                        .w_full()
                        .h(px(4.0))
                        .cursor_row_resize()
                        .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, _, _| {
                                this.drag_coll =
                                    Some((f32::from(event.position.y), this.collections_h));
                            }),
                        ),
                )
            })
            .child(
                div()
                    .id("explorer-history-area")
                    .flex_1()
                    .w_full()
                    .overflow_scroll()
                    .child(self.render_history_section(cx)),
            )
            .when(self.env_editor_open, |el| {
                el.child(
                    div()
                        .id("drag-handle-env")
                        .w_full()
                        .h(px(4.0))
                        .cursor_row_resize()
                        .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, _, _| {
                                this.drag_env = Some((f32::from(event.position.y), this.env_h));
                            }),
                        ),
                )
            })
            .child(self.render_environment_section(cx))
            .when(self.env_col_drag.is_some(), |el| {
                el.child(
                    deferred(
                        div()
                            .id("env-col-resize-overlay")
                            .absolute()
                            .top_0()
                            .left_0()
                            .w_full()
                            .h_full()
                            .cursor_col_resize()
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                                if let Some((start_x, start_w)) = this.env_col_drag {
                                    let delta = f32::from(event.position.x) - start_x;
                                    this.env_col_key_w = (start_w + delta).clamp(40.0, 300.0);
                                    cx.notify();
                                }
                            }))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.env_col_drag = None;
                                    cx.notify();
                                }),
                            ),
                    )
                    .with_priority(1),
                )
            })
            .when(self.drag_coll.is_some(), |el| {
                el.child(
                    deferred(
                        div()
                            .id("drag-coll-overlay")
                            .absolute()
                            .inset_0()
                            .cursor_row_resize()
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                                if let Some((start_y, start_h)) = this.drag_coll {
                                    let delta = f32::from(event.position.y) - start_y;
                                    this.collections_h = (start_h + delta).clamp(48.0, 600.0);
                                    cx.notify();
                                }
                            }))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.drag_coll = None;
                                    crate::prefs::set_f32(
                                        "explorer.collections_h",
                                        this.collections_h,
                                    );
                                    cx.notify();
                                }),
                            ),
                    )
                    .with_priority(2),
                )
            })
            .when(self.drag_env.is_some(), |el| {
                el.child(
                    deferred(
                        div()
                            .id("drag-env-overlay")
                            .absolute()
                            .inset_0()
                            .cursor_row_resize()
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                                if let Some((start_y, start_h)) = this.drag_env {
                                    let delta = f32::from(event.position.y) - start_y;
                                    this.env_h = (start_h - delta).clamp(80.0, 500.0);
                                    cx.notify();
                                }
                            }))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.drag_env = None;
                                    crate::prefs::set_f32("explorer.env_h", this.env_h);
                                    cx.notify();
                                }),
                            ),
                    )
                    .with_priority(2),
                )
            })
            .when_some(self.context_menu.clone(), |el, (path, position)| {
                el.child(
                    div()
                        .id("context-menu-backdrop")
                        .absolute()
                        .inset_0()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.close_context_menu(cx);
                            }),
                        )
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(|this, _, _, cx| {
                                this.close_context_menu(cx);
                            }),
                        ),
                )
                .child(self.render_context_menu(path, position, cx))
            })
    }
}
