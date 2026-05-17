//! tRPC tab rendering for RequestPanel


use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, SharedString, Styled,
};

use crate::theme;
use crate::ui::components::icons::{icon, ICON_SM, ICON_CLOSE};
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::{EditTarget, TrpcBatchCall};
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

    pub(super) fn render_trpc_batch_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let selected_idx = self.trpc_selected_batch_idx;
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();

        // Pre-collect call data to avoid borrow conflicts in the loop
        let calls_data: Vec<(String, bool, bool)> = self.trpc_batch_calls.iter().enumerate()
            .map(|(i, c)| (c.procedure.clone(), c.enabled, Some(i) == selected_idx))
            .collect();
        let call_count = calls_data.len();

        let selected_label = selected_idx
            .and_then(|i| self.trpc_batch_calls.get(i))
            .map(|c| if c.procedure.is_empty() { "(unnamed)".to_string() } else { c.procedure.clone() });

        let mut list = div()
            .w_full()
            .flex()
            .flex_col()
            .py(px(4.0))
            .track_focus(&self.edit_focus);

        for (i, (procedure, enabled, is_selected)) in calls_data.into_iter().enumerate() {
            let is_editing = active_edit == Some(EditTarget::TrpcBatchProcedure(i));
            let sel = if is_editing { edit_selection.clone() } else { 0..0 };
            let can_remove = i + 1 < call_count || !procedure.is_empty();

            let row = div()
                .id(SharedString::from(format!("trpc-batch-row-{}", i)))
                .w_full()
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(8.0))
                .py(px(3.0))
                .when(is_selected, |el| el.bg(theme.colors.accent.opacity(0.08)))
                .when(!is_selected, |el| el.hover(|s| s.bg(theme.colors.hover_overlay)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_batch_call(i, cx);
                }))
                // Enable checkbox
                .child(
                    div()
                        .id(SharedString::from(format!("trpc-batch-enable-{}", i)))
                        .size(px(16.0))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(call) = this.trpc_batch_calls.get_mut(i) {
                                call.enabled = !call.enabled;
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .size(px(12.0))
                                .border_1()
                                .when(enabled, |el| el
                                    .bg(theme.colors.accent)
                                    .border_color(theme.colors.accent)
                                )
                                .when(!enabled, |el| el
                                    .border_color(theme.colors.border)
                                )
                        )
                )
                // Procedure name
                .child(
                    self.render_kv_input_flex(
                        format!("trpc-batch-proc-{}", i),
                        EditTarget::TrpcBatchProcedure(i),
                        &procedure,
                        "procedure.name",
                        is_editing,
                        sel,
                        cx,
                    )
                )
                // Remove button
                .when(can_remove, |el| {
                    el.child(
                        div()
                            .id(SharedString::from(format!("trpc-batch-remove-{}", i)))
                            .size(px(20.0))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_muted)
                            .hover(|s| s.text_color(theme.colors.status_client_error))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_batch_call(i, cx);
                            }))
                            .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted))
                    )
                });

            list = list.child(row);
        }

        div()
            .id("trpc-batch-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            // Header
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .py(px(8.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("Batch Procedures")
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("POST {base}/{proc1},{proc2}?batch=1  (tRPC v11 native format)")
                            )
                    )
                    .child(
                        div()
                            .id("trpc-batch-add")
                            .px(px(10.0))
                            .py(px(4.0))
                            .bg(theme.colors.accent.opacity(0.1))
                            .border_1()
                            .border_color(theme.colors.accent.opacity(0.3))
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.accent)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.accent.opacity(0.2)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.trpc_batch_calls.push(TrpcBatchCall { enabled: true, ..Default::default() });
                                cx.notify();
                            }))
                            .child("+ Add")
                    )
            )
            // Call list
            .child(list)
            // Params editor
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .min_h(px(120.0))
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(12.0))
                            .py(px(6.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_secondary)
                                    .child("PARAMS")
                            )
                            .when_some(selected_label, |el, label| {
                                el.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(theme.colors.text_muted)
                                        .child(format!("for {}", label))
                                )
                            })
                            .when(selected_idx.is_none(), |el| {
                                el.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(theme.colors.text_muted)
                                        .child("select a procedure above to edit its parameters")
                                )
                            })
                    )
                    .when(selected_idx.is_some(), |el| {
                        el.child(
                            div()
                                .flex()
                                .flex_1()
                                .w_full()
                                .border_1()
                                .border_color(theme.colors.border)
                                .overflow_hidden()
                                .child(self.trpc_batch_params_editor.clone())
                        )
                    })
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
