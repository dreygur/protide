use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*, px};

use super::super::request_types::BodyType;
use super::RequestPanel;
use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_body_type_btn_form(
        &self,
        id: &'static str,
        label: &'static str,
        bt: BodyType,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_active = self.body_type == bt;
        div()
            .id(id)
            .h_full()
            .px(px(16.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .border_b_2()
            .when(is_active, |el| el.border_color(theme.colors.accent))
            .when(!is_active, |el| {
                el.border_color(gpui::transparent_black())
                    .hover(|s| s.bg(theme.colors.hover_overlay))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_body_type(bt, cx);
            }))
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(if is_active {
                        gpui::FontWeight::MEDIUM
                    } else {
                        gpui::FontWeight::NORMAL
                    })
                    .text_color(if is_active {
                        theme.colors.text_primary
                    } else {
                        theme.colors.text_secondary
                    })
                    .child(label),
            )
    }
}
