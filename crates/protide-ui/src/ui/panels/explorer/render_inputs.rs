use gpui::{AnyElement, Context, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, SharedString, Styled, canvas, div, px};
use super::*;

impl ExplorerPanel {
    pub(super) fn render_text_input_w(
        &mut self,
        id: impl Into<SharedString>,
        target: EnvEditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: std::ops::Range<usize>,
        width: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(id.into())
            .w(px(width))
            .min_w(px(40.0))
            .h(px(28.0))
            .px(px(6.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
            .bg(theme.colors.bg_tertiary)
            .text_size(px(11.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, target, 6.6, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 6.6, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
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
                                .insert(target, f32::from(bounds.origin.x) + 6.0);
                            this.edit_input_widths
                                .insert(target, f32::from(bounds.size.width));
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
                let max_chars = (((width - 12.0) / 6.6).max(1.0)) as usize;
                let scroll = if is_editing {
                    self.edit_scroll_offsets.get(&target).copied().unwrap_or(0.0)
                } else {
                    0.0
                };
                render_text_view_with_max_scrolled(
                    &text,
                    &selection,
                    is_editing,
                    11.0,
                    theme.colors.text_primary,
                    Some(placeholder),
                    theme.colors.text_muted,
                    Some(max_chars),
                    theme.colors.accent.opacity(0.25),
                    scroll,
                )
            })
    }

    pub(super) fn render_text_input(
        &mut self,
        id: impl Into<SharedString>,
        target: EnvEditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: std::ops::Range<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(id.into())
            .flex_1()
            .min_w(px(0.0))
            .h(px(28.0))
            .px(px(6.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
            .bg(theme.colors.bg_tertiary)
            .text_size(px(11.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, target, 6.6, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 6.6, cx);
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
                                .insert(target, f32::from(bounds.origin.x) + 6.0);
                            this.edit_input_widths
                                .insert(target, f32::from(bounds.size.width));
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
                let scroll = if is_editing {
                    self.edit_scroll_offsets.get(&target).copied().unwrap_or(0.0)
                } else {
                    0.0
                };
                self.render_text_content_scrolled(text, placeholder, is_editing, selection, scroll, cx)
            })
    }

    pub(super) fn render_text_content_scrolled(
        &self,
        text: String,
        placeholder: &'static str,
        is_focused: bool,
        selection: std::ops::Range<usize>,
        scroll_offset_x: f32,
        cx: &Context<Self>,
    ) -> AnyElement {
        let theme = theme::current(cx);
        render_text_view_with_max_scrolled(
            &text,
            &selection,
            is_focused,
            11.0,
            theme.colors.text_primary,
            Some(placeholder),
            theme.colors.text_muted,
            None,
            theme.colors.accent.opacity(0.25),
            scroll_offset_x,
        )
    }
}
