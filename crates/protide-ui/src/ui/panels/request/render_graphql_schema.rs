//! GraphQL schema tab rendering for RequestPanel


use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, SharedString, Styled,
};

use crate::theme;
use crate::ui::components::icons::{
    icon, ICON_SM, ICON_MD,
    ICON_FILE,
};
use protide_core::execution::ws::WebSocketExecutor;
use super::{GraphqlSchemaState, RequestPanel};

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_graphql_schema_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        use crate::ui::components::icons::{ICON_REFRESH, ICON_SEARCH, ICON_LOADER};
        let theme = theme::current(cx);
        let schema = self.graphql_schema.clone();
        let search = self.graphql_schema_search.clone();

        let toolbar = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(4.0))
            .pb(px(8.0))
            .child(
                div()
                    .id("gql-schema-refresh")
                    .h(px(28.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .bg(theme.colors.accent.opacity(0.1))
                    .border_1()
                    .border_color(theme.colors.accent.opacity(0.3))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.accent.opacity(0.18)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.fetch_graphql_schema(cx);
                    }))
                    .child(icon(ICON_REFRESH, ICON_SM, theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.colors.accent)
                            .child("Refresh Schema"),
                    )
            )
            .child(
                div()
                    .id("gql-schema-import")
                    .h(px(28.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .bg(theme.colors.bg_secondary)
                    .border_1()
                    .border_color(theme.colors.border)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.import_graphql_schema_file(cx);
                    }))
                    .child(icon(ICON_FILE, ICON_SM, theme.colors.text_muted))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Import File"),
                    )
            );

        let body: gpui::AnyElement = match &schema {
            GraphqlSchemaState::Idle => div()
                .w_full()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(theme.colors.text_muted)
                                .child("No schema loaded."),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted.opacity(0.7))
                                .child("Use \"Refresh Schema\" to fetch via introspection,"),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted.opacity(0.7))
                                .child("or \"Import File\" to load a .graphql / .json schema."),
                        )
                )
                .into_any_element(),

            GraphqlSchemaState::Loading => div()
                .w_full()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .child(icon(ICON_LOADER, ICON_MD, theme.colors.accent))
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.colors.text_muted)
                        .child("Fetching schema…"),
                )
                .into_any_element(),

            GraphqlSchemaState::Error(msg) => {
                let msg = msg.clone();
                div()
                    .w_full()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.error)
                                    .child("Introspection failed"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(SharedString::from(msg)),
                            )
                    )
                    .into_any_element()
            }

            GraphqlSchemaState::Loaded(types) => {
                let types = types.clone();
                let search_lower = search.to_lowercase();
                let filtered: Vec<_> = types
                    .iter()
                    .filter(|t| search_lower.is_empty() || t.name.to_lowercase().contains(&search_lower))
                    .cloned()
                    .collect();

                div()
                    .w_full()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .id("gql-schema-search-box")
                            .w_full()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .px(px(8.0))
                            .bg(theme.colors.bg_secondary)
                            .border_1()
                            .border_color(theme.colors.border)
                            .child(icon(ICON_SEARCH, ICON_SM, theme.colors.text_muted))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.0))
                                    .text_color(if search.is_empty() {
                                        theme.colors.text_muted
                                    } else {
                                        theme.colors.text_primary
                                    })
                                    .child(if search.is_empty() {
                                        SharedString::from("Filter types…")
                                    } else {
                                        SharedString::from(search.clone())
                                    })
                            )
                    )
                    .child(
                        div()
                            .px(px(4.0))
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child(SharedString::from(format!(
                                "{} type{}",
                                filtered.len(),
                                if filtered.len() == 1 { "" } else { "s" }
                            )))
                    )
                    .child(
                        div()
                            .w_full()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .id("gql-schema-type-list")
                            .overflow_scroll()
                            .children(filtered.iter().enumerate().map(|(i, t)| {
                                let kind_color = match t.kind.as_str() {
                                    "OBJECT"    => theme.colors.method_post,
                                    "INTERFACE" => theme.colors.accent,
                                    "ENUM"      => theme.colors.method_patch,
                                    "INPUT_OBJECT" | "INPUT" => theme.colors.method_put,
                                    "SCALAR"    => theme.colors.text_muted,
                                    _           => theme.colors.text_secondary,
                                };
                                div()
                                    .id(SharedString::from(format!("gql-type-{}", i)))
                                    .w_full()
                                    .h(px(26.0))
                                    .flex()
                                    .items_center()
                                    .px(px(8.0))
                                    .gap(px(8.0))
                                    .hover(|s| s.bg(theme.colors.hover_overlay))
                                    .child(
                                        div()
                                            .px(px(4.0))
                                            .py(px(1.0))
                                            .min_w(px(52.0))
                                            .flex()
                                            .justify_center()
                                            .bg(kind_color.opacity(0.12))
                                            .text_size(px(9.0))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(kind_color)
                                            .child(SharedString::from(t.kind.clone()))
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.text_primary)
                                            .child(SharedString::from(t.name.clone()))
                                    )
                            }))
                    )
                    .into_any_element()
            }
        };

        div()
            .id("graphql-schema-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(toolbar)
            .child(body)
            .into_any_element()
    }
}
