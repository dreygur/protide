//! Tab bar and tab content routing for RequestPanel


use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, SharedString, Styled,
};

use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::RequestMode;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let tab_labels: &[&str] = match self.request_mode {
            RequestMode::GraphQL => &["Query", "Variables", "Headers", "Auth", "Schema"],
            RequestMode::WebSocket => &["Messages", "Headers"],
            RequestMode::SocketIo => &["Events", "Headers"],
            RequestMode::Grpc => &["Message", "Metadata", "Proto"],
            RequestMode::Trpc => &["Procedure", "Parameters", "Headers", "Auth"],
            RequestMode::Http => &["Params", "Headers", "Body", "Auth", "Scripts", "Data"],
        };
        let active_tab = self.active_tab;
        let param_count = self.params.iter().filter(|p| p.enabled && !p.key.is_empty()).count();
        let header_count = self.headers.iter().filter(|h| h.enabled && !h.key.is_empty()).count();
        let mode = self.request_mode;

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
                let count = match mode {
                    RequestMode::GraphQL => if i == 2 { header_count } else { 0 },
                    RequestMode::WebSocket => if i == 1 { header_count } else { 0 },
                    RequestMode::SocketIo => if i == 1 { header_count } else { 0 },
                    RequestMode::Grpc => if i == 1 { header_count } else { 0 },
                    RequestMode::Trpc => if i == 2 { header_count } else { 0 },
                    RequestMode::Http => match i {
                        0 => param_count,
                        1 => header_count,
                        _ => 0,
                    },
                };

                div()
                    .id(SharedString::from(format!("req-tab-{}", i)))
                    .h_full()
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_pointer()
                    .border_b_2()
                    .when(is_active, |el| el.border_color(theme.colors.accent))
                    .when(!is_active, |el| {
                        el.border_color(gpui::transparent_black())
                            .hover(|s| s.bg(theme.colors.hover_overlay))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_tab(i, cx);
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
                            .child(*label)
                    )
                    .when(count > 0, |el| {
                        el.child(
                            div()
                                .px(px(5.0))
                                .py(px(1.0))
                                .bg(theme.colors.accent.opacity(0.15))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.accent)
                                .child(format!("{}", count))
                        )
                    })
            }))
    }

    pub(super) fn render_tab_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.request_mode {
            RequestMode::GraphQL => {
                match self.active_tab {
                    0 => self.render_graphql_query_tab(cx),
                    1 => self.render_graphql_variables_tab(cx),
                    2 => self.render_headers_tab(cx),
                    3 => self.render_auth_tab(cx),
                    4 => self.render_graphql_schema_tab(cx),
                    _ => div().into_any_element(),
                }
            }
            RequestMode::WebSocket => {
                match self.active_tab {
                    0 => self.render_websocket_messages_tab(cx),
                    1 => self.render_headers_tab(cx),
                    _ => div().into_any_element(),
                }
            }
            RequestMode::SocketIo => {
                match self.active_tab {
                    0 => self.render_socketio_events_tab(cx),
                    1 => self.render_headers_tab(cx),
                    _ => div().into_any_element(),
                }
            }
            RequestMode::Http => {
                match self.active_tab {
                    0 => self.render_params_tab(cx),
                    1 => self.render_headers_tab(cx),
                    2 => self.render_body_tab(cx),
                    3 => self.render_auth_tab(cx),
                    4 => self.render_scripts_tab(cx),
                    5 => self.render_data_tab(cx),
                    _ => div().into_any_element(),
                }
            }
            RequestMode::Grpc => {
                match self.active_tab {
                    0 => self.render_grpc_message_tab(cx),
                    1 => self.render_grpc_metadata_tab(cx),
                    2 => self.render_grpc_proto_tab(cx),
                    _ => div().into_any_element(),
                }
            }
            RequestMode::Trpc => {
                match self.active_tab {
                    0 => self.render_trpc_procedure_tab(cx),
                    1 => self.render_trpc_parameters_tab(cx),
                    2 => self.render_headers_tab(cx),
                    3 => self.render_auth_tab(cx),
                    _ => div().into_any_element(),
                }
            }
        }
    }
}
