//! gRPC message and metadata tab rendering for RequestPanel


use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, SharedString, Styled,
};

use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::{EditTarget, GrpcStreamingType};
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_grpc_message_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let has_service = self.grpc_service.is_some();
        let has_method = self.grpc_method.is_some();

        div()
            .id("grpc-message-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            // Service/Method selection row
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
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
                                    .child("Service")
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(px(32.0))
                                    .px(px(12.0))
                                    .bg(theme.colors.bg_secondary)
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.0))
                                    .text_color(if has_service { theme.colors.text_primary } else { theme.colors.text_muted })
                                    .child(
                                        self.grpc_service.clone()
                                            .unwrap_or_else(|| "Select service...".to_string())
                                    )
                            )
                    )
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
                                    .child("Method")
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(px(32.0))
                                    .px(px(12.0))
                                    .bg(theme.colors.bg_secondary)
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(if has_method { theme.colors.text_primary } else { theme.colors.text_muted })
                                    .child(
                                        self.grpc_method
                                            .as_ref()
                                            .map(|m| m.full_name.clone())
                                            .unwrap_or_else(|| "Select method...".to_string())
                                    )
                                    .when(self.grpc_method.as_ref().map(|m| m.streaming_type != GrpcStreamingType::Unary).unwrap_or(false), |el| {
                                        el.child(
                                            div()
                                                .id("grpc-streaming-badge")
                                                .px(px(6.0))
                                                .py(px(2.0))
                                                .bg(theme.colors.protocol_grpc.opacity(0.15))
                                                .text_size(px(9.0))
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(theme.colors.protocol_grpc)
                                                .child(
                                                    self.grpc_method.as_ref()
                                                        .map(|m| m.streaming_type.label())
                                                        .unwrap_or("")
                                                )
                                        )
                                    })
                            )
                    )
            )
            // Message editor
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
                            .child("Request Message (JSON)")
                    )
                    .child(
                        div()
                            .flex_1()
                            .border_1()
                            .border_color(theme.colors.border)
                            .overflow_hidden()
                            .child(self.grpc_message_editor.clone())
                    )
            )
            .when(self.grpc_proto_path.is_none(), |el| {
                el.child(
                    div()
                        .w_full()
                        .py(px(8.0))
                        .px(px(12.0))
                        .bg(theme.colors.accent.opacity(0.1))
                        .text_size(px(11.0))
                        .text_color(theme.colors.text_muted)
                        .child("Load a .proto file in the Proto tab to select services and methods")
                )
            })
            .into_any_element()
    }

    pub(super) fn render_grpc_metadata_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let meta_len = self.grpc_metadata.len();
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();
        let enabled_count = self.grpc_metadata.iter().filter(|m| m.enabled && !m.key.is_empty()).count();

        let meta_data: Vec<_> = self.grpc_metadata.iter().enumerate().map(|(i, m)| {
            (i, m.enabled, m.key.clone(), m.value.clone())
        }).collect();

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .track_focus(&self.edit_focus);

        container = container.child(self.render_kv_table_header("KEY", "VALUE", enabled_count, cx));

        for (i, is_enabled, key, value) in meta_data {
            let can_remove = meta_len > 1;
            let is_editing_key = active_edit == Some(EditTarget::GrpcMetaKey(i));
            let is_editing_value = active_edit == Some(EditTarget::GrpcMetaValue(i));
            let is_row_editing = is_editing_key || is_editing_value;

            container = container.child(
                div()
                    .id(SharedString::from(format!("grpc-meta-row-{}", i)))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(4.0))
                    .px(px(2.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    .child(
                        self.render_kv_checkbox(format!("grpc-meta-cb-{}", i).into(), is_enabled, cx)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_grpc_meta(i, cx);
                            }))
                    )
                    .child(
                        self.render_kv_input(
                            format!("grpc-meta-key-{}", i),
                            EditTarget::GrpcMetaKey(i),
                            &key,
                            "metadata-key",
                            is_editing_key,
                            if is_editing_key { edit_selection.clone() } else { 0..0 },
                            px(self.kv_col_key_w),
                            cx,
                        )
                    )
                    .child(div().w(px(4.0)))
                    .child(
                        self.render_kv_input_flex(
                            format!("grpc-meta-val-{}", i),
                            EditTarget::GrpcMetaValue(i),
                            &value,
                            "value",
                            is_editing_value,
                            if is_editing_value { edit_selection.clone() } else { 0..0 },
                            cx,
                        )
                    )
                    .child(
                        self.render_kv_remove_btn(format!("grpc-meta-del-{}", i).into(), can_remove, cx)
                            .when(can_remove, |el| el.on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_grpc_meta(i, cx);
                            })))
                    )
            );
        }

        container = container.child(
            div().w_full().pt(px(8.0)).child(
                self.render_kv_add_btn("add-grpc-meta-btn", "+ Add Metadata", cx)
                    .on_click(cx.listener(|this, _, _, cx| { this.add_grpc_meta(cx); }))
            )
        );

        container.into_any_element()
    }
}
