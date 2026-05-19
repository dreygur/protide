//! gRPC proto tab rendering for RequestPanel

use std::ops::Range;

use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, Styled,
};

use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_grpc_proto_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let has_proto = self.grpc_proto_path.is_some();
        let proto_path = self.grpc_proto_path.clone()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        div()
            .id("grpc-proto-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            // Load proto button row
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .id("load-proto-btn")
                            .px(px(16.0))
                            .py(px(8.0))
                            .bg(theme.colors.accent)
                            .cursor_pointer()
                            .hover(|s| s.opacity(0.9))
                            .text_size(px(12.0))
                            .text_color(theme.colors.bg_primary)
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.load_proto_file(cx);
                            }))
                            .child("Load .proto file")
                    )
                    .when(has_proto, |el| {
                        el.child(
                            div()
                                .flex_1()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted)
                                .overflow_x_hidden()
                                .child(proto_path)
                        )
                        .child(
                            div()
                                .id("clear-proto-btn")
                                .px(px(12.0))
                                .py(px(8.0))
                                .bg(theme.colors.bg_tertiary)
                                .cursor_pointer()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted)
                                .hover(|s| s.bg(theme.colors.method_delete.opacity(0.1)).text_color(theme.colors.method_delete))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.grpc_proto_path = None;
                                    this.grpc_proto_content.clear();
                                    this.grpc_services.clear();
                                    this.grpc_service = None;
                                    this.grpc_methods.clear();
                                    this.grpc_method = None;
                                    cx.notify();
                                }))
                                .child("Clear")
                        )
                    })
            )
            // Proto content display
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child("Proto Definition")
                    )
                    .child(
                        div()
                            .id("proto-content-scroll")
                            .flex_1()
                            .w_full()
                            .border_1()
                            .border_color(theme.colors.border)
                            .bg(theme.colors.bg_secondary)
                            .p(px(12.0))
                            .overflow_scroll()
                            .when(has_proto, |el| {
                                el.child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(theme.colors.text_primary)
                                        .font_family("monospace")
                                        .child(self.grpc_proto_content.clone())
                                )
                            })
                            .when(!has_proto, |el| {
                                el.child(
                                    div()
                                        .w_full()
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_muted)
                                        .child("No proto file loaded")
                                )
                            })
                    )
            )
            .into_any_element()
    }
}
