//! Key-value table helpers for RequestPanel

use std::ops::Range;

use gpui::{
    canvas, div, prelude::*, px, Context, IntoElement, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, SharedString, Styled,
};

use crate::theme;
use crate::components::{render_text_view_with_max};
use crate::components::icons::{
    icon, ICON_SM,
    ICON_CLOSE, ICON_CHECK,
};
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::EditTarget;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Drag handle between KEY and VALUE column headers in KV tables
    pub(super) fn render_kv_col_drag_handle(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let start_w = self.kv_col_key_w;
        div()
            .id("kv-col-drag-handle")
            .w(px(4.0))
            .self_stretch()
            .cursor_col_resize()
            .bg(theme.colors.border.opacity(0.6))
            .hover(|s| s.bg(theme.colors.accent.opacity(0.6)))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                this.kv_col_drag = Some((f32::from(event.position.x), start_w));
                cx.notify();
            }))
    }

    pub(super) fn render_count_badge(&self, count: usize, suffix: &str, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        div()
            .px(px(6.0)).py(px(2.0))
            .bg(theme.colors.accent.opacity(0.12))
            .border_1().border_color(theme.colors.accent.opacity(0.35))
            .text_size(px(10.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.colors.accent)
            .child(format!("{} {}", count, suffix))
    }

    pub(super) fn render_kv_table_header(&self, key_label: &'static str, value_label: &'static str, count: usize, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        div()
            .w_full().flex().items_center()
            .gap(px(8.0))
            .py(px(6.0))
            .border_b_1().border_color(theme.colors.border)
            .mb(px(4.0))
            .child(div().size(px(16.0)))
            .child(
                div()
                    .w(px(self.kv_col_key_w))
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.colors.accent.opacity(0.7))
                    .child(key_label)
            )
            .child(self.render_kv_col_drag_handle(cx))
            .child(
                div().flex_1().flex().items_center().justify_between()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_secondary)
                            .child(value_label)
                    )
                    .child(self.render_count_badge(count, "active", cx))
            )
            .child(div().size(px(28.0)))
    }

    pub(super) fn render_kv_checkbox(&self, id: SharedString, is_enabled: bool, cx: &Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = theme::current(cx);
        div()
            .id(id)
            .size(px(16.0))
            .border_1()
            .cursor_pointer()
            .when(is_enabled, |el| el.bg(theme.colors.accent).border_color(theme.colors.accent))
            .when(!is_enabled, |el| {
                el.border_color(theme.colors.border)
                    .hover(|s| s.border_color(theme.colors.text_muted))
            })
            .flex().items_center().justify_center()
            .when(is_enabled, |el| {
                el.child(div().flex().items_center().justify_center().child(icon(ICON_CHECK, ICON_SM, theme.colors.bg_primary)))
            })
    }

    pub(super) fn render_kv_remove_btn(&self, id: SharedString, can_remove: bool, cx: &Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = theme::current(cx);
        div()
            .id(id)
            .size(px(28.0))
            .flex().items_center().justify_center()
            .text_size(px(14.0))
            .when(can_remove, |el| {
                el.cursor_pointer()
                    .text_color(theme.colors.text_muted)
                    .hover(|s| s.bg(theme.colors.status_client_error.opacity(0.1)).text_color(theme.colors.status_client_error))
            })
            .when(!can_remove, |el| {
                el.cursor(gpui::CursorStyle::Arrow)
                    .text_color(theme.colors.border)
            })
            .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted))
    }

    pub(super) fn render_kv_add_btn(&self, id: &'static str, label: &'static str, cx: &Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = theme::current(cx);
        div()
            .id(id)
            .w_full()
            .py(px(8.0))
            .border_1()
            .border_color(theme.colors.border.opacity(0.5))
            .flex().items_center().justify_center()
            .cursor_pointer()
            .text_size(px(12.0))
            .text_color(theme.colors.text_muted)
            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.border).text_color(theme.colors.text_secondary))
            .child(label)
    }

    /// Render a key-value input field with fixed width
    pub(super) fn render_kv_input(
        &mut self,
        id: String,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        width: gpui::Pixels,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(SharedString::from(id))
            .w(width)
            .min_w(px(0.0))
            .when(!is_editing, |el| el.h(px(28.0)).overflow_hidden())
            .when(is_editing, |el| el.min_h(px(28.0)).py(px(4.0)))
            .px(px(8.0))
            .flex()
            .when(!is_editing, |el| el.items_center())
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| {
                el.border_color(gpui::transparent_black())
                  .hover(|s| s.border_color(theme.colors.border))
            })
            .bg(theme.colors.bg_tertiary)
            .text_size(px(12.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.skip_blur = true;
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, target, 7.2, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 7.2, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_edit_mouse_up(event, cx);
                }),
            )
            .child({
                let entity = cx.entity();
                canvas(
                    move |bounds, _, cx| {
                        let _ = entity.update(cx, |this, _| {
                            this.edit_input_origins.insert(target, f32::from(bounds.origin.x) + 8.0);
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute().top_0().left_0().size_full()
            })
            .child({
                let max_chars = (((f32::from(width) - 16.0) / 7.2).max(1.0)) as usize;
                self.render_kv_text(&text, placeholder, is_editing, selection, Some(max_chars), cx)
            })
    }

    /// Render a key-value input field with flex width
    pub(super) fn render_kv_input_flex(
        &mut self,
        id: String,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(SharedString::from(id))
            .flex_1()
            .min_w(px(0.0))
            .when(!is_editing, |el| el.h(px(28.0)).overflow_hidden())
            .when(is_editing, |el| el.min_h(px(28.0)).py(px(4.0)))
            .px(px(8.0))
            .flex()
            .when(!is_editing, |el| el.items_center())
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| {
                el.border_color(gpui::transparent_black())
                  .hover(|s| s.border_color(theme.colors.border))
            })
            .bg(theme.colors.bg_tertiary)
            .text_size(px(12.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.skip_blur = true;
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, target, 7.2, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 7.2, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_edit_mouse_up(event, cx);
                }),
            )
            .child({
                let entity = cx.entity();
                canvas(
                    move |bounds, _, cx| {
                        let _ = entity.update(cx, |this, _| {
                            this.edit_input_origins.insert(target, f32::from(bounds.origin.x) + 9.0);
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute().top_0().left_0().size_full()
            })
            .child(self.render_kv_text(&text, placeholder, is_editing, selection, None, cx))
    }

    /// Render text with cursor/selection for kv inputs
    pub(super) fn render_kv_text(
        &self,
        text: &str,
        placeholder: &'static str,
        is_focused: bool,
        selection: Range<usize>,
        max_chars: Option<usize>,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        render_text_view_with_max(
            text,
            &selection,
            is_focused,
            12.0,
            theme.colors.text_primary,
            Some(placeholder),
            theme.colors.text_muted,
            max_chars,
            theme.colors.accent.opacity(0.25),
        )
    }
}
