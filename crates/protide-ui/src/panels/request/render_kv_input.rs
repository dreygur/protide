use std::ops::Range;

use gpui::{
    Context, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement,
    SharedString, Styled, canvas, div, prelude::*, px,
};

use super::super::request_types::EditTarget;
use super::RequestPanel;
use crate::components::render_text_view_with_max;
use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;

impl<E: WebSocketExecutor> RequestPanel<E> {
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
                        entity.update(cx, |this, _| {
                            this.edit_input_origins
                                .insert(target, f32::from(bounds.origin.x) + 8.0);
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full()
            })
            .child({
                let max_chars = (((f32::from(width) - 16.0) / 7.2).max(1.0)) as usize;
                self.render_kv_text(
                    &text,
                    placeholder,
                    is_editing,
                    selection,
                    Some(max_chars),
                    cx,
                )
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
                        entity.update(cx, |this, _| {
                            this.edit_input_origins
                                .insert(target, f32::from(bounds.origin.x) + 9.0);
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full()
            })
            .child(self.render_kv_text(&text, placeholder, is_editing, selection, None, cx))
    }
}
