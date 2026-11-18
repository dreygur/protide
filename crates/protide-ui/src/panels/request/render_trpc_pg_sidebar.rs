//! tRPC Playground — procedure list and add-procedure row rendering

use super::super::request_types::{PendingEditor, TrpcPlaygroundProc, TrpcProcKind};
use super::RequestPanel;
use crate::components::icons::{ICON_PLUS, ICON_SM, icon};
use crate::theme;
use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*, px};
use gpui_component::{Sizable, input::Input};
use protide_core::execution::ws::WebSocketExecutor;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_pg_proc_list(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let search = self.trpc_pg_search_input.read(cx).value().to_string();
        let q = search.to_lowercase();

        let filtered: Vec<usize> = self
            .trpc_pg_procedures
            .iter()
            .enumerate()
            .filter(|(_, p)| q.is_empty() || p.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();

        let mut routers: Vec<String> = Vec::new();
        for &idx in &filtered {
            let r = self.trpc_pg_procedures[idx].router();
            let key = if r.is_empty() {
                "(root)".to_string()
            } else {
                r.to_string()
            };
            if !routers.contains(&key) {
                routers.push(key);
            }
        }

        let selected = self.trpc_pg_selected;
        let editing = self.trpc_pg_editing;
        let edit_kind = self.trpc_pg_edit_kind;

        div()
            .id("trpc-pg-proc-list")
            .flex_1()
            .w_full()
            .overflow_scroll()
            .flex()
            .flex_col()
            .children(routers.into_iter().map(|router| {
                let procs: Vec<usize> = filtered
                    .iter()
                    .copied()
                    .filter(|&i| {
                        let r = self.trpc_pg_procedures[i].router();
                        (if r.is_empty() { "(root)" } else { r }) == router.as_str()
                    })
                    .collect();

                // Group header — normal or edit mode
                let group_header = self.render_pg_group_header(router, cx);

                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .child(group_header)
                    .children(
                        procs.into_iter().map(|idx| {
                            self.render_pg_proc_row(idx, selected, editing, edit_kind, cx)
                        }),
                    )
            }))
            .into_any_element()
    }

    pub(super) fn render_pg_add_row(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let add_kind = self.trpc_pg_add_kind;

        let chip = |id: &'static str, lbl: &'static str, active: bool, bc, bg, tc| {
            div()
                .id(id)
                .h(px(24.0))
                .w(px(22.0))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .border_1()
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(active, |el| el.border_color(bc).bg(bg).text_color(tc))
                .when(!active, |el| {
                    el.border_color(theme.colors.border)
                        .bg(theme.colors.bg_secondary)
                        .text_color(theme.colors.text_muted)
                })
                .child(lbl)
        };
        div()
            .flex_none()
            .h(px(40.0))
            .border_t_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_primary)
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(6.0))
            .child(
                chip(
                    "trpc-pg-kind-q",
                    "Q",
                    add_kind == TrpcProcKind::Query,
                    theme.colors.method_get,
                    theme.colors.method_get.opacity(0.12),
                    theme.colors.method_get,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.trpc_pg_add_kind = TrpcProcKind::Query;
                    cx.notify();
                })),
            )
            .child(
                chip(
                    "trpc-pg-kind-m",
                    "M",
                    add_kind == TrpcProcKind::Mutation,
                    theme.colors.method_post,
                    theme.colors.method_post.opacity(0.12),
                    theme.colors.method_post,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.trpc_pg_add_kind = TrpcProcKind::Mutation;
                    cx.notify();
                })),
            )
            .child(
                div()
                    .flex_1()
                    .h(px(26.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    .child(
                        Input::new(&self.trpc_pg_add_input)
                            .bordered(false)
                            .with_size(gpui_component::Size::XSmall),
                    ),
            )
            .child(
                div()
                    .id("trpc-pg-add-btn")
                    .h(px(26.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .bg(theme.colors.accent.opacity(0.08))
                    .border_1()
                    .border_color(theme.colors.accent.opacity(0.3))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.accent.opacity(0.16)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let name = this.trpc_pg_add_input.read(cx).value().trim().to_string();
                        if !name.is_empty() {
                            let kind = this.trpc_pg_add_kind;
                            this.trpc_pg_procedures
                                .push(TrpcPlaygroundProc { kind, name });
                            this.queue_editor(PendingEditor::TrpcPgAddInput, String::new());
                            cx.notify();
                        }
                    }))
                    .child(icon(ICON_PLUS, ICON_SM, theme.colors.accent)),
            )
            .into_any_element()
    }
}
