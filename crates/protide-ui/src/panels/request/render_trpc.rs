//! tRPC tab rendering for RequestPanel

use std::ops::Range;

use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, Styled,
};

use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::EditTarget;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_trpc_procedure_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let procedure = self.trpc_procedure.clone();
        let is_editing = self.active_edit == Some(EditTarget::TrpcProcedure);
        let edit_selection = self.edit_selection.clone();

        div()
            .id("trpc-procedure-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .p(px(16.0))
            .track_focus(&self.edit_focus)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("tRPC Procedure")
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.colors.text_muted)
                            .child("Enter the procedure name (e.g., \"query.getUser\", \"mutation.createPost\")")
                    )
                    .child(
                        self.render_kv_input_flex(
                            "trpc-procedure-input".to_string(),
                            EditTarget::TrpcProcedure,
                            &procedure,
                            "query.getUser",
                            is_editing,
                            if is_editing { edit_selection } else { 0..0 },
                            cx,
                        )
                    )
            )
            .into_any_element()
    }

    pub(super) fn render_trpc_parameters_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        div()
            .id("trpc-parameters-tab")
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
                            .bg(theme.colors.method_post.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.0))
                            .text_color(theme.colors.method_post)
                            .child("{ }")
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Parameters (JSON)")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Enter the parameters for your tRPC procedure")
                    )
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    .child(self.trpc_params_editor.clone())
            )
            .into_any_element()
    }
}
