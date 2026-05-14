//! Auth content dispatch and Bearer rendering for RequestPanel

use std::ops::Range;

use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, Styled,
};

use crate::theme;
use crate::ui::components::icons::{
    icon, ICON_SM, ICON_MD,
    ICON_FORM, ICON_CIRCLE_X,
};
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::{AuthType, EditTarget};
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_auth_content(
        &mut self,
        auth_type: AuthType,
        active_edit: Option<EditTarget>,
        edit_selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        match auth_type {
            AuthType::None => render_auth_none(&theme).into_any_element(),
            AuthType::Bearer => self.render_bearer_content(active_edit, edit_selection, cx),
            AuthType::Basic => self.render_basic_content(active_edit, edit_selection, cx),
            AuthType::ApiKey => self.render_apikey_content(active_edit, edit_selection, cx),
        }
    }

    pub(super) fn render_bearer_content(
        &mut self,
        active_edit: Option<EditTarget>,
        edit_selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let token = self.bearer_token.clone();
        let is_editing = active_edit == Some(EditTarget::BearerToken);

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
                            .child(icon(ICON_FORM, ICON_MD, theme.colors.accent))
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
                                    .child("Bearer Token")
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("Authorization: Bearer <token>")
                            )
                    )
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.text_secondary)
                            .child("TOKEN")
                    )
                    .child(
                        self.render_auth_input(
                            "bearer-token",
                            EditTarget::BearerToken,
                            &token,
                            "Enter bearer token...",
                            is_editing,
                            if is_editing { edit_selection } else { 0..0 },
                            cx,
                        )
                    )
            )
            .into_any_element()
    }
}

fn render_auth_none(theme: &crate::theme::Theme) -> gpui::Div {
    div()
        .w_full()
        .p(px(16.0))
        .bg(theme.colors.bg_tertiary.opacity(0.5))
        .border_1()
        .border_color(theme.colors.border.opacity(0.5))
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .size(px(32.0))
                .bg(theme.colors.text_muted.opacity(0.1))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(16.0))
                .text_color(theme.colors.text_muted)
                .child(icon(ICON_CIRCLE_X, ICON_SM, theme.colors.text_muted))
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .p(px(20.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.colors.text_secondary)
                        .child("No Authentication")
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.colors.text_muted)
                        .child("Request will be sent without auth headers")
                )
        )
}
