//! Basic and API Key auth content rendering for RequestPanel

use std::ops::Range;

use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, Styled,
};

use crate::theme;
use crate::ui::components::icons::{
    icon, ICON_MD,
    ICON_USER, ICON_KEY,
};
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::{ApiKeyLocation, EditTarget};
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_basic_content(
        &mut self,
        active_edit: Option<EditTarget>,
        edit_selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let username = self.basic_username.clone();
        let password = self.basic_password.clone();
        let is_editing_user = active_edit == Some(EditTarget::BasicUsername);
        let is_editing_pass = active_edit == Some(EditTarget::BasicPassword);

        div()
            .w_full()
            .p(px(16.0))
            .bg(theme.colors.bg_tertiary.opacity(0.5))
            .border_1()
            .border_color(theme.colors.border.opacity(0.5))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        div()
                            .size(px(32.0))
                            .bg(theme.colors.accent.opacity(0.1))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(ICON_USER, ICON_MD, theme.colors.accent))
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_primary)
                                    .child("Basic Authentication")
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("Authorization: Basic <base64>")
                            )
                    )
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("USERNAME")
                            )
                            .child(
                                self.render_auth_input(
                                    "basic-username",
                                    EditTarget::BasicUsername,
                                    &username,
                                    "Enter username...",
                                    is_editing_user,
                                    if is_editing_user { edit_selection.clone() } else { 0..0 },
                                    cx,
                                )
                            )
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("PASSWORD")
                            )
                            .child(
                                self.render_auth_input_masked(
                                    "basic-password",
                                    EditTarget::BasicPassword,
                                    &password,
                                    "Enter password...",
                                    is_editing_pass,
                                    if is_editing_pass { edit_selection } else { 0..0 },
                                    cx,
                                )
                            )
                    )
            )
            .into_any_element()
    }

    pub(super) fn render_apikey_content(
        &mut self,
        active_edit: Option<EditTarget>,
        edit_selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let key_name = self.api_key_name.clone();
        let key_value = self.api_key_value.clone();
        let location = self.api_key_location;
        let is_editing_name = active_edit == Some(EditTarget::ApiKeyName);
        let is_editing_value = active_edit == Some(EditTarget::ApiKeyValue);

        div()
            .w_full()
            .p(px(16.0))
            .bg(theme.colors.bg_tertiary.opacity(0.5))
            .border_1()
            .border_color(theme.colors.border.opacity(0.5))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(render_api_key_header(&theme, location))
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("KEY NAME")
                            )
                            .child(
                                self.render_auth_input(
                                    "api-key-name",
                                    EditTarget::ApiKeyName,
                                    &key_name,
                                    "e.g., X-API-Key",
                                    is_editing_name,
                                    if is_editing_name { edit_selection.clone() } else { 0..0 },
                                    cx,
                                )
                            )
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("VALUE")
                            )
                            .child(
                                self.render_auth_input_masked(
                                    "api-key-value",
                                    EditTarget::ApiKeyValue,
                                    &key_value,
                                    "Enter API key...",
                                    is_editing_value,
                                    if is_editing_value { edit_selection } else { 0..0 },
                                    cx,
                                )
                            )
                    )
            )
            .child(render_apikey_location_selector(&theme, location, cx))
            .into_any_element()
    }
}

fn render_api_key_header(theme: &crate::theme::Theme, location: ApiKeyLocation) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(
            div()
                .size(px(32.0))
                .bg(theme.colors.accent.opacity(0.1))
                .flex()
                .items_center()
                .justify_center()
                .child(icon(ICON_KEY, ICON_MD, theme.colors.accent))
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.colors.text_primary)
                        .child("API Key")
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(theme.colors.text_muted)
                        .when(location == ApiKeyLocation::Header, |el| {
                            el.child("Sent as request header")
                        })
                        .when(location == ApiKeyLocation::QueryParam, |el| {
                            el.child("Sent as query parameter")
                        })
                )
        )
}

fn render_apikey_location_selector<E: WebSocketExecutor>(
    theme: &crate::theme::Theme,
    location: ApiKeyLocation,
    cx: &mut gpui::Context<super::RequestPanel<E>>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.colors.text_secondary)
                .child("ADD TO")
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
                        .child("Header")
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
                        .child("Query Param")
                )
        )
}
