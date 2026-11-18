use super::super::request_types::{PendingEditor, TrpcProcKind};
use super::RequestPanel;
use crate::components::icons::{ICON_CHECK, ICON_EDIT, ICON_SM, icon};
use crate::theme;
use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*, px};
use gpui_component::{Sizable, input::Input};
use protide_core::execution::ws::WebSocketExecutor;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_pg_proc_row(
        &self,
        idx: usize,
        selected: Option<usize>,
        editing: Option<usize>,
        edit_kind: TrpcProcKind,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let kind = self.trpc_pg_procedures[idx].kind;
        let is_sel = selected == Some(idx);
        let is_editing_row = editing == Some(idx);
        let kc = match kind {
            TrpcProcKind::Query => theme.colors.method_get,
            TrpcProcKind::Mutation => theme.colors.method_post,
        };

        if is_editing_row {
            div()
                .id(format!("trpc-pg-row-{}", idx))
                .h(px(30.0))
                .pl(px(8.0))
                .pr(px(8.0))
                .flex()
                .items_center()
                .gap(px(4.0))
                .border_b_1()
                .border_color(theme.colors.border.opacity(0.5))
                .bg(theme.colors.accent.opacity(0.04))
                // Q toggle
                .child(
                    div()
                        .id(format!("trpc-pg-ek-q-{}", idx))
                        .w(px(16.0))
                        .h(px(16.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(if edit_kind == TrpcProcKind::Query {
                            theme.colors.method_get.opacity(0.12)
                        } else {
                            theme.colors.bg_secondary
                        })
                        .text_size(px(9.0))
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(if edit_kind == TrpcProcKind::Query {
                            theme.colors.method_get
                        } else {
                            theme.colors.text_muted
                        })
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.trpc_pg_edit_kind = TrpcProcKind::Query;
                            cx.notify();
                        }))
                        .child("Q"),
                )
                // M toggle
                .child(
                    div()
                        .id(format!("trpc-pg-ek-m-{}", idx))
                        .w(px(16.0))
                        .h(px(16.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(if edit_kind == TrpcProcKind::Mutation {
                            theme.colors.method_post.opacity(0.12)
                        } else {
                            theme.colors.bg_secondary
                        })
                        .text_size(px(9.0))
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(if edit_kind == TrpcProcKind::Mutation {
                            theme.colors.method_post
                        } else {
                            theme.colors.text_muted
                        })
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.trpc_pg_edit_kind = TrpcProcKind::Mutation;
                            cx.notify();
                        }))
                        .child("M"),
                )
                // Name input
                .child(
                    div().flex_1().h_full().overflow_hidden().child(
                        Input::new(&self.trpc_pg_edit_input)
                            .bordered(false)
                            .with_size(gpui_component::Size::XSmall),
                    ),
                )
                // Confirm
                .child(
                    div()
                        .id(format!("trpc-pg-ok-{}", idx))
                        .size(px(16.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                let n = this.trpc_pg_edit_input.read(cx).value().trim().to_string();
                                if !n.is_empty()
                                    && let Some(proc) = this.trpc_pg_procedures.get_mut(idx)
                                {
                                    proc.name = n;
                                    proc.kind = this.trpc_pg_edit_kind;
                                }
                                this.trpc_pg_editing = None;
                                cx.notify();
                            }),
                        )
                        .child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success)),
                )
                // Cancel
                .child(
                    div()
                        .id(format!("trpc-pg-cl-{}", idx))
                        .size(px(14.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .cursor_pointer()
                        .hover(|s| s.text_color(theme.colors.status_client_error))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.trpc_pg_editing = None;
                                cx.notify();
                            }),
                        )
                        .child("×"),
                )
                .into_any_element()
        } else {
            let leaf = self
                .trpc_pg_procedures
                .get(idx)
                .map(|p| p.leaf().to_string())
                .unwrap_or_default();
            div()
                .id(format!("trpc-pg-row-{}", idx))
                .h(px(30.0))
                .pl(px(22.0))
                .pr(px(8.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .cursor_pointer()
                .border_b_1()
                .border_color(theme.colors.border.opacity(0.5))
                .when(is_sel, |el| {
                    el.bg(theme.colors.accent.opacity(0.08))
                        .border_l_2()
                        .border_color(theme.colors.accent)
                })
                .when(!is_sel, |el| el.hover(|s| s.bg(theme.colors.hover_overlay)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(proc) = this.trpc_pg_procedures.get(idx) {
                        this.trpc_pg_selected = Some(idx);
                        this.trpc_procedure = proc.full_procedure();
                    }
                    cx.notify();
                }))
                // Kind badge
                .child(
                    div()
                        .w(px(16.0))
                        .h(px(16.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(kc.opacity(0.12))
                        .text_size(px(9.0))
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(kc)
                        .child(match kind {
                            TrpcProcKind::Query => "Q",
                            TrpcProcKind::Mutation => "M",
                        }),
                )
                // Leaf name
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.0))
                        .font_family("JetBrains Mono")
                        .text_color(if is_sel {
                            theme.colors.text_primary
                        } else {
                            theme.colors.text_secondary
                        })
                        .child(leaf),
                )
                // Edit button
                .child(
                    div()
                        .id(format!("trpc-pg-ed-{}", idx))
                        .size(px(14.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(theme.colors.text_muted.opacity(0.0))
                        .hover(|s| s.text_color(theme.colors.accent))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                if let Some(proc) = this.trpc_pg_procedures.get(idx) {
                                    let n = proc.name.clone();
                                    let k = proc.kind;
                                    this.trpc_pg_editing = Some(idx);
                                    this.trpc_pg_edit_kind = k;
                                    this.queue_editor(PendingEditor::TrpcPgEditInput, n);
                                    cx.notify();
                                }
                            }),
                        )
                        .child(icon(ICON_EDIT, ICON_SM, theme.colors.text_muted)),
                )
                // Delete button
                .child(
                    div()
                        .id(format!("trpc-pg-del-{}", idx))
                        .size(px(14.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted.opacity(0.0))
                        .hover(|s| s.text_color(theme.colors.status_client_error))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.trpc_pg_procedures.remove(idx);
                                match this.trpc_pg_selected {
                                    Some(s) if s == idx => this.trpc_pg_selected = None,
                                    Some(s) if s > idx => this.trpc_pg_selected = Some(s - 1),
                                    _ => {}
                                }
                                cx.notify();
                            }),
                        )
                        .child("×"),
                )
                .into_any_element()
        }
    }
}
