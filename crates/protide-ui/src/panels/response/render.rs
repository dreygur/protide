use super::*;

impl Render for ResponsePanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some((text, lang)) = self.body_pending.take() {
            self.body_viewer.update(cx, |s, cx| {
                s.set_value(&text, window, cx);
                s.set_highlighter(&lang, cx);
            });
        }
        let theme = theme::current(cx);
        let is_col_dragging = self.resp_col_drag.is_some();
        let has_ctx_menu = self.json_context_menu.is_some();
        let entity = cx.entity();

        let has_response = self.response.is_some();
        let context_menu_pos = self.context_menu_pos;

        div()
            .id("response-panel-root")
            .size_full()
            .flex()
            .flex_col()
            .relative()
            .bg(theme.colors.bg_primary)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, _, cx| {
                if this.context_menu_pos.is_some() {
                    this.context_menu_pos = None;
                    cx.notify();
                }
            }))
            // Dismiss body context menu on left-click anywhere
            .when(context_menu_pos.is_some(), |el| {
                el.on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                    this.context_menu_pos = None;
                    cx.notify();
                }))
            })
            // Capture panel origin for JSON context-menu local-coord conversion
            .child(
                canvas(
                    move |bounds: Bounds<Pixels>, _win, cx| {
                        let _ = entity.update(cx, |this, _| {
                            this.bounds_origin = bounds.origin;
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .child(self.render_header(cx))
            .child(self.render_tabs(cx))
            .child(
                div()
                    .id("response-content")
                    .flex_1()
                    .w_full()
                    .p(px(12.0))
                    .overflow_scroll()
                    .relative()
                    .track_scroll(&self.content_scroll_handle)
                    // Right-click to show copy context menu
                    .when(has_response, |el| {
                        el.on_mouse_down(
                            MouseButton::Right,
                            cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                this.context_menu_pos = Some(event.position);
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        )
                    })
                    .child(self.render_content(cx))
                    .vertical_scrollbar(&self.content_scroll_handle),
            )
            // Body-level right-click context menu (copy / clear)
            .when_some(context_menu_pos, |el, pos| {
                let body = self.response.as_ref().map(|r| r.body.clone()).unwrap_or_default();
                let x = f32::from(pos.x);
                let y = f32::from(pos.y);
                el.child(deferred(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(y))
                        .w(px(160.0))
                        .bg(theme.colors.bg_secondary)
                        .border_1()
                        .border_color(theme.colors.border)
                        .shadow_lg()
                        .overflow_hidden()
                        .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                            cx.stop_propagation();
                        }))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                // Copy response body
                                .child({
                                    let b = body.clone();
                                    div()
                                        .id("resp-ctx-copy")
                                        .w_full().h(px(28.0))
                                        .flex().items_center()
                                        .px(px(12.0)).gap(px(8.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                            cx.write_to_clipboard(ClipboardItem::new_string(b.clone()));
                                            this.context_menu_pos = None;
                                            cx.notify();
                                        }))
                                        .child(icon(ICON_COPY, ICON_SM, theme.colors.text_muted))
                                        .child(div().text_size(px(12.0)).text_color(theme.colors.text_primary).child("Copy Response"))
                                })
                                // Clear response
                                .child(
                                    div()
                                        .id("resp-ctx-clear")
                                        .w_full().h(px(28.0))
                                        .flex().items_center()
                                        .px(px(12.0)).gap(px(8.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                            this.response = None;
                                            this.context_menu_pos = None;
                                            cx.notify();
                                        }))
                                        .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted))
                                        .child(div().text_size(px(12.0)).text_color(theme.colors.text_primary).child("Clear"))
                                )
                        )
                ).with_priority(10))
            })
            // JSON tree right-click context menu overlay
            .when(has_ctx_menu, |el| {
                el.child(deferred(self.render_json_context_menu(cx)).with_priority(10))
            })
            // Column resize overlay
            .when(is_col_dragging, |el| el.child(
                deferred(
                    div()
                        .id("resp-col-resize-overlay")
                        .absolute().top_0().left_0().w_full().h_full()
                        .cursor_col_resize()
                        .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                            if let Some((drag_id, start_x, start_w)) = this.resp_col_drag {
                                let delta = f32::from(event.position.x) - start_x;
                                let new_w = (start_w + delta).max(60.0);
                                let changed = match drag_id {
                                    0 => { let w = new_w.min(600.0); let c = (this.resp_header_col1_w - w).abs() > 0.5; this.resp_header_col1_w = w; c }
                                    1 => { let w = new_w.min(400.0); let c = (this.cookie_col1_w - w).abs() > 0.5; this.cookie_col1_w = w; c }
                                    2 => { let w = new_w.min(300.0); let c = (this.cookie_col3_w - w).abs() > 0.5; this.cookie_col3_w = w; c }
                                    3 => { let w = new_w.min(200.0); let c = (this.cookie_col4_w - w).abs() > 0.5; this.cookie_col4_w = w; c }
                                    _ => false,
                                };
                                if changed { cx.notify(); }
                            }
                        }))
                        .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.resp_col_drag = None;
                            cx.notify();
                        }))
                ).with_priority(2)
            ))
    }
}
