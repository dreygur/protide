//! Auth tab rendering for RequestPanel

use std::ops::Range;

use gpui::{
    canvas, div, prelude::*, px, Context, IntoElement, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, SharedString, Styled,
};

use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::{AuthType, EditTarget};
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_auth_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let auth_type = self.auth_type;
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();

        let auth_types = [
            (AuthType::None, "None"),
            (AuthType::Bearer, "Bearer"),
            (AuthType::Basic, "Basic"),
            (AuthType::ApiKey, "API Key"),
        ];

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .track_focus(&self.edit_focus)
            .child(
                div()
                    .h(px(40.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_primary)
                    .children(auth_types.iter().map(|(at, label)| {
                        let is_selected = *at == auth_type;
                        let at = *at;
                        div()
                            .id(SharedString::from(format!("auth-type-{:?}", at)))
                            .h_full()
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .when(is_selected, |el| el.border_color(theme.colors.accent))
                            .when(!is_selected, |el| {
                                el.border_color(gpui::transparent_black())
                                    .hover(|s| s.bg(theme.colors.hover_overlay))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_auth_type(at, cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(if is_selected {
                                        gpui::FontWeight::MEDIUM
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .text_color(if is_selected {
                                        theme.colors.text_primary
                                    } else {
                                        theme.colors.text_secondary
                                    })
                                    .child(*label)
                            )
                    }))
            )
            .child(self.render_auth_content(auth_type, active_edit, edit_selection, cx))
            .into_any_element()
    }

    pub(super) fn render_auth_input(
        &mut self,
        id: &str,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.render_auth_input_impl(id, target, text, placeholder, is_editing, selection, false, cx)
    }

    pub(super) fn render_auth_input_masked(
        &mut self,
        id: &str,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.render_auth_input_impl(id, target, text, placeholder, is_editing, selection, true, cx)
    }

    pub(super) fn render_auth_input_impl(
        &mut self,
        id: &str,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        masked: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let display_text = if masked && !text.is_empty() {
            "●".repeat(text.len())
        } else {
            text.to_string()
        };

        div()
            .id(SharedString::from(id.to_string()))
            .w_full()
            .max_w(px(400.0))
            .min_w(px(0.0))
            .when(!is_editing, |el| el.h(px(32.0)).overflow_hidden())
            .when(is_editing, |el| el.min_h(px(32.0)).py(px(6.0)))
            .px(px(12.0))
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
                            this.edit_input_origins.insert(target, f32::from(bounds.origin.x) + 12.0);
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute().top_0().left_0().size_full()
            })
            .child(self.render_kv_text(&display_text, placeholder, is_editing, selection, Some(50), cx))
    }
}
