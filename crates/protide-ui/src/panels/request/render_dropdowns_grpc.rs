use gpui::{Context, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px};

use super::super::request_types::GrpcStreamingType;
use super::RequestPanel;
use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_grpc_service_dropdown_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let services = self.grpc_services.clone();
        let selected = self.grpc_service.clone();

        div()
            .id("grpc-service-dropdown-overlay")
            .absolute()
            .top(px(64.0))
            .left(px(0.0))
            .w(px(240.0))
            .max_h(px(240.0))
            .overflow_scroll()
            .py(px(6.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.skip_blur = true;
                    cx.stop_propagation();
                }),
            )
            .children(services.into_iter().map(|svc| {
                let is_selected = Some(&svc) == selected.as_ref();
                let svc_click = svc.clone();
                div()
                    .id(SharedString::from(format!("grpc-service-{}", svc)))
                    .debug_selector({
                        let svc = svc.clone();
                        move || format!("grpc-service-{}", svc)
                    })
                    .mx(px(4.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .cursor_pointer()
                    .when(is_selected, |el| el.bg(theme.colors.accent.opacity(0.1)))
                    .when(!is_selected, |el| {
                        el.hover(|s| s.bg(theme.colors.bg_tertiary))
                    })
                    .text_size(px(12.0))
                    .text_color(if is_selected {
                        theme.colors.text_primary
                    } else {
                        theme.colors.text_secondary
                    })
                    .child(svc)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_grpc_service(svc_click.clone(), cx);
                    }))
            }))
    }

    pub(super) fn render_grpc_method_dropdown_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let prefix = self
            .grpc_service
            .clone()
            .map(|s| format!("{}/", s))
            .unwrap_or_default();
        let methods: Vec<_> = self
            .grpc_methods
            .iter()
            .filter(|m| m.full_name.starts_with(&prefix))
            .cloned()
            .collect();
        let selected = self.grpc_method.clone();

        div()
            .id("grpc-method-dropdown-overlay")
            .absolute()
            .top(px(64.0))
            .left(px(230.0))
            .w(px(240.0))
            .max_h(px(240.0))
            .overflow_scroll()
            .py(px(6.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.skip_blur = true;
                    cx.stop_propagation();
                }),
            )
            .children(methods.into_iter().map(|m| {
                let is_selected = selected
                    .as_ref()
                    .map(|s| s.full_name == m.full_name)
                    .unwrap_or(false);
                let short_name = m
                    .full_name
                    .rsplit('/')
                    .next()
                    .unwrap_or(&m.full_name)
                    .to_string();
                let m_click = m.clone();
                div()
                    .id(SharedString::from(format!("grpc-method-{}", m.full_name)))
                    .debug_selector({
                        let full_name = m.full_name.clone();
                        move || format!("grpc-method-{}", full_name)
                    })
                    .mx(px(4.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .cursor_pointer()
                    .when(is_selected, |el| el.bg(theme.colors.accent.opacity(0.1)))
                    .when(!is_selected, |el| {
                        el.hover(|s| s.bg(theme.colors.bg_tertiary))
                    })
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(if is_selected {
                                theme.colors.text_primary
                            } else {
                                theme.colors.text_secondary
                            })
                            .child(short_name),
                    )
                    .when(m.streaming_type != GrpcStreamingType::Unary, |el| {
                        el.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .bg(theme.colors.protocol_grpc.opacity(0.15))
                                .text_size(px(9.0))
                                .text_color(theme.colors.protocol_grpc)
                                .child(m.streaming_type.label()),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_grpc_method(m_click.clone(), cx);
                    }))
            }))
    }
}
