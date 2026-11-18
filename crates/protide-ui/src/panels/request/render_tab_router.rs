//! Tab bar and tab content routing for RequestPanel

use gpui::{Context, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px};

use super::super::request_types::{AuthType, RequestMode};
use super::RequestPanel;
use crate::components::tab_item;
use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let tab_labels = self.request_mode.tab_labels();
        let active_tab = self.active_tab;
        let param_count = self
            .params
            .iter()
            .filter(|p| p.enabled && !p.key.is_empty())
            .count();
        let header_count = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .count();
        let auth_count = usize::from(self.auth_type != AuthType::None);

        div()
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_primary)
            .children(tab_labels.iter().enumerate().map(|(i, label)| {
                let is_active = i == active_tab;
                let count = match *label {
                    "Params" => param_count,
                    "Headers" | "Metadata" => header_count,
                    "Auth" => auth_count,
                    _ => 0,
                };

                tab_item(
                    SharedString::from(format!("req-tab-{}", i)),
                    *label,
                    is_active,
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_tab(i, cx);
                }))
                .when(count > 0, |el| {
                    el.child(
                        div()
                            .px(px(5.0))
                            .py(px(1.0))
                            .bg(theme.colors.accent.opacity(0.15))
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.accent)
                            .child(format!("{}", count)),
                    )
                })
            }))
    }

    pub(super) fn render_tab_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.request_mode {
            RequestMode::GraphQL => match self.active_tab {
                0 => self.render_graphql_query_tab(cx),
                1 => self.render_graphql_variables_tab(cx),
                2 => self.render_headers_tab(cx),
                3 => self.render_auth_tab(cx),
                4 => self.render_graphql_schema_tab(cx),
                _ => div().into_any_element(),
            },
            RequestMode::WebSocket => match self.active_tab {
                0 => self.render_websocket_messages_tab(cx),
                1 => self.render_headers_tab(cx),
                _ => div().into_any_element(),
            },
            RequestMode::SocketIo => match self.active_tab {
                0 => self.render_socketio_events_tab(cx),
                1 => self.render_headers_tab(cx),
                _ => div().into_any_element(),
            },
            RequestMode::Http => match self.active_tab {
                0 => self.render_params_tab(cx),
                1 => self.render_headers_tab(cx),
                2 => self.render_body_tab(cx),
                3 => self.render_auth_tab(cx),
                4 => self.render_scripts_tab(cx),
                5 => self.render_data_tab(cx),
                6 => self.render_settings_tab(cx),
                _ => div().into_any_element(),
            },
            RequestMode::Grpc => match self.active_tab {
                0 => self.render_grpc_message_tab(cx),
                1 => self.render_grpc_metadata_tab(cx),
                2 => self.render_grpc_proto_tab(cx),
                _ => div().into_any_element(),
            },
            RequestMode::Trpc => match self.active_tab {
                0 => self.render_trpc_playground_tab(cx),
                1 => self.render_headers_tab(cx),
                2 => self.render_auth_tab(cx),
                _ => div().into_any_element(),
            },
        }
    }
}
