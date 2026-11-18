use gpui::{ParentElement, Styled, div, prelude::*, px};

use super::super::request_types::ApiKeyLocation;
use protide_core::execution::ws::WebSocketExecutor;

pub(super) fn render_apikey_location_selector<E: WebSocketExecutor>(
    theme: &crate::theme::Theme,
    location: ApiKeyLocation,
    cx: &mut gpui::Context<super::RequestPanel<E>>,
) -> gpui::Div {
    div()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.colors.text_secondary)
                .child("ADD TO"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .p(px(3.0))
                .bg(theme.colors.bg_primary)
                .child(
                    div()
                        .id("api-key-header")
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .px(px(10.0))
                        .py(px(5.0))
                        .cursor_pointer()
                        .text_size(px(11.0))
                        .when(location == ApiKeyLocation::Header, |el| {
                            el.bg(theme.colors.bg_tertiary)
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.text_primary)
                                .child("H")
                        })
                        .when(location != ApiKeyLocation::Header, |el| {
                            el.text_color(theme.colors.text_muted)
                                .hover(|s| s.text_color(theme.colors.text_secondary))
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.api_key_location != ApiKeyLocation::Header {
                                this.toggle_api_key_location(cx);
                            }
                        }))
                        .child("Header"),
                )
                .child(
                    div()
                        .id("api-key-query")
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .px(px(10.0))
                        .py(px(5.0))
                        .cursor_pointer()
                        .text_size(px(11.0))
                        .when(location == ApiKeyLocation::QueryParam, |el| {
                            el.bg(theme.colors.bg_tertiary)
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.text_primary)
                                .child("?")
                        })
                        .when(location != ApiKeyLocation::QueryParam, |el| {
                            el.text_color(theme.colors.text_muted)
                                .hover(|s| s.text_color(theme.colors.text_secondary))
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.api_key_location != ApiKeyLocation::QueryParam {
                                this.toggle_api_key_location(cx);
                            }
                        }))
                        .child("Query Param"),
                ),
        )
}
