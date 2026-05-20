//! GraphQL query and variables tab rendering for RequestPanel


use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, Styled,
};

use crate::theme;
use crate::components::icons::{
    icon, ICON_SM,
    ICON_PLAY,
};
use protide_core::execution::ws::WebSocketExecutor;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_graphql_query_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        div()
            .id("graphql-query-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(4.0))
                    .child(
                        div()
                            .size(px(20.0))
                            .bg(theme.colors.method_delete.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(ICON_PLAY, ICON_SM, theme.colors.method_delete))
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("GraphQL Query")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Write your query, mutation, or subscription")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h(px(200.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    .child(self.graphql_query_editor.clone())
            )
            .into_any_element()
    }

    pub(super) fn render_graphql_variables_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        div()
            .id("graphql-variables-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(4.0))
                    .child(
                        div()
                            .size(px(20.0))
                            .bg(theme.colors.accent.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.0))
                            .text_color(theme.colors.accent)
                            .child("{ }")
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Variables")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("JSON object with query variables")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h(px(150.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    .child(self.graphql_variables_editor.clone())
            )
            .into_any_element()
    }
}
