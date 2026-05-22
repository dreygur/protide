//! Render trait implementation for RequestPanel

use gpui::{
    deferred, div, prelude::*, px, Context, IntoElement, KeyDownEvent,
    MouseButton, MouseMoveEvent, ParentElement, Render, Styled, Window,
};
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::{KvList, RequestMode};
use super::RequestPanel;

impl<E: WebSocketExecutor> Render for RequestPanel<E> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.skip_blur = false;
        self.apply_pending_editors(window, cx);
        div()
            .id("request-panel")
            .size_full().flex().flex_col().relative()
            .track_focus(&self.body_focus)
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.control && event.keystroke.key == "s" {
                    this.save_request(cx); return;
                }
                if event.keystroke.key == "escape" {
                    if this.mode_dropdown_open { this.mode_dropdown_open = false; cx.notify(); return; }
                    if this.method_dropdown_open { this.method_dropdown_open = false; cx.notify(); return; }
                }
                if this.active_edit.is_some() { this.handle_edit_key(event, cx); }
            }))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                if !this.skip_blur && this.active_edit.is_some() { this.active_edit = None; cx.notify(); }
                if this.method_dropdown_open { this.method_dropdown_open = false; cx.notify(); }
                if this.mode_dropdown_open { this.mode_dropdown_open = false; cx.notify(); }
            }))
            .child(self.render_url_bar(window, cx))
            .child(self.render_tabs(cx))
            .child({
                let is_pg = self.request_mode == RequestMode::Trpc && self.active_tab == 0;
                let base = div().id("tab-content").flex_1().w_full();
                if is_pg { base.overflow_hidden() } else { base.p(px(12.0)).overflow_scroll() }
                    .child(self.render_tab_content(cx))
            })
            .when(self.method_dropdown_open, |el|
                el.child(deferred(self.render_method_dropdown_overlay(window, cx)).with_priority(1)))
            .when(self.mode_dropdown_open, |el|
                el.child(deferred(self.render_mode_dropdown_overlay(cx)).with_priority(1)))
            .when(self.kv_col_drag.is_some(), |el| el.child(deferred(
                div().id("kv-col-resize-overlay")
                    .absolute().top_0().left_0().w_full().h_full()
                    .cursor_col_resize()
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                        if let Some((start_x, start_w)) = this.kv_col_drag {
                            let new_w = (start_w + f32::from(event.position.x) - start_x).max(60.0).min(500.0);
                            if (this.kv_col_key_w - new_w).abs() > 0.5 { this.kv_col_key_w = new_w; cx.notify(); }
                        }
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        this.kv_col_drag = None; cx.notify();
                    }))
            ).with_priority(1)))
            .when(self.kv_row_drag.is_some(), |el| el.child(deferred(
                div().id("kv-row-drag-overlay")
                    .absolute().top_0().left_0().w_full().h_full()
                    .cursor_grab()
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                        if let Some((list, src, start_y)) = this.kv_row_drag {
                            let len = match list {
                                KvList::Params => this.params.len(),
                                KvList::Headers => this.headers.len(),
                                KvList::GrpcMeta => this.grpc_metadata.len(),
                            };
                            if len > 1 {
                                let delta = f32::from(event.position.y) - start_y;
                                let idx = (src as i32 + (delta / 36.0).round() as i32)
                                    .clamp(0, len as i32 - 1) as usize;
                                if this.kv_row_drag_over != Some(idx) {
                                    this.kv_row_drag_over = Some(idx);
                                    cx.notify();
                                }
                            }
                        }
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        let drag = this.kv_row_drag.take();
                        let over = this.kv_row_drag_over.take();
                        if let (Some((list, src, _)), Some(dst)) = (drag, over) {
                            if src != dst { this.reorder_kv(list, src, dst, cx); }
                        }
                        cx.notify();
                    }))
            ).with_priority(2)))
            .when(self.form_row_drag.is_some(), |el| el.child(deferred(
                div().id("form-row-drag-overlay")
                    .absolute().top_0().left_0().w_full().h_full()
                    .cursor_grab()
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                        if let Some((src, start_y)) = this.form_row_drag {
                            let len = this.form_data.len();
                            if len > 1 {
                                let delta = f32::from(event.position.y) - start_y;
                                let idx = (src as i32 + (delta / 36.0).round() as i32)
                                    .clamp(0, len as i32 - 1) as usize;
                                if this.form_row_drag_over != Some(idx) {
                                    this.form_row_drag_over = Some(idx);
                                    cx.notify();
                                }
                            }
                        }
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        let drag = this.form_row_drag.take();
                        let over = this.form_row_drag_over.take();
                        if let (Some((src, _)), Some(dst)) = (drag, over) {
                            if src != dst { this.reorder_form_field(src, dst, cx); }
                        }
                        cx.notify();
                    }))
            ).with_priority(2)))
    }
}
