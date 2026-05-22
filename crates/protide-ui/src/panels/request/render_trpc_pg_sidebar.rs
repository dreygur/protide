//! tRPC Playground — procedure list and add-procedure row rendering

use gpui::{
    div, prelude::*, px, Context, IntoElement, ParentElement, SharedString, Styled,
};
use gpui_component::input::Input;
use crate::theme;
use crate::components::icons::{icon, ICON_SM, ICON_CHEVRON_DOWN, ICON_PLUS};
use super::super::request_types::{TrpcProcKind, TrpcPlaygroundProc, PendingEditor};
use protide_core::execution::ws::WebSocketExecutor;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_pg_proc_list(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let search = self.trpc_pg_search_input.read(cx).value().to_string();
        let q = search.to_lowercase();

        let filtered: Vec<usize> = self.trpc_pg_procedures.iter().enumerate()
            .filter(|(_, p)| q.is_empty() || p.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();

        // Collect unique routers in first-seen order
        let mut routers: Vec<String> = Vec::new();
        for &idx in &filtered {
            let r = self.trpc_pg_procedures[idx].router();
            let key = if r.is_empty() { "(root)".to_string() } else { r.to_string() };
            if !routers.contains(&key) { routers.push(key); }
        }

        let selected = self.trpc_pg_selected;

        div()
            .id("trpc-pg-proc-list")
            .flex_1()
            .w_full()
            .overflow_scroll()
            .flex()
            .flex_col()
            .children(routers.into_iter().map(|router| {
                let procs: Vec<usize> = filtered.iter().copied()
                    .filter(|&idx| {
                        let r = self.trpc_pg_procedures[idx].router();
                        (if r.is_empty() { "(root)" } else { r }) == router.as_str()
                    })
                    .collect();

                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    // Router group header
                    .child(
                        div()
                            .h(px(22.0))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .bg(theme.colors.bg_primary)
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .child(icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .font_family("JetBrains Mono")
                                    .child(router.clone())
                            )
                    )
                    // Procedure rows
                    .children(procs.into_iter().map(|idx| {
                        let proc = &self.trpc_pg_procedures[idx];
                        let kind = proc.kind;
                        let leaf = proc.leaf().to_string();
                        let is_sel = selected == Some(idx);
                        let (kind_col, kind_lbl) = match kind {
                            TrpcProcKind::Query    => (theme.colors.method_get,  "Q"),
                            TrpcProcKind::Mutation => (theme.colors.method_post, "M"),
                        };

                        div()
                            .id(SharedString::from(format!("trpc-pg-row-{}", idx)))
                            .h(px(30.0))
                            .pl(px(22.0))
                            .pr(px(8.0))
                            .flex()
                            .items_center()
                            .gap(px(7.0))
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
                                this.trpc_pg_selected = Some(idx);
                                let full = this.trpc_pg_procedures[idx].full_procedure();
                                this.trpc_procedure = full;
                                cx.notify();
                            }))
                            // Kind badge (Q / M)
                            .child(
                                div()
                                    .w(px(16.0))
                                    .h(px(16.0))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(kind_col.opacity(0.12))
                                    .text_size(px(9.0))
                                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                    .text_color(kind_col)
                                    .child(kind_lbl)
                            )
                            // Leaf procedure name
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
                                    .child(leaf)
                            )
                            // Delete button (stops click propagation to row)
                            .child(
                                div()
                                    .id(SharedString::from(format!("trpc-pg-del-{}", idx)))
                                    .size(px(14.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_muted.opacity(0.0))
                                    .hover(|s| s.text_color(theme.colors.status_client_error))
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.trpc_pg_procedures.remove(idx);
                                        match this.trpc_pg_selected {
                                            Some(s) if s == idx => this.trpc_pg_selected = None,
                                            Some(s) if s > idx  => this.trpc_pg_selected = Some(s - 1),
                                            _ => {}
                                        }
                                        cx.notify();
                                    }))
                                    .child("×")
                            )
                    }))
            }))
    }

    pub(super) fn render_pg_add_row(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let add_kind = self.trpc_pg_add_kind;

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
            // Q chip
            .child(
                div()
                    .id("trpc-pg-kind-q")
                    .h(px(24.0))
                    .w(px(22.0))
                    .flex().items_center().justify_center()
                    .cursor_pointer()
                    .border_1()
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .when(add_kind == TrpcProcKind::Query, |el| {
                        el.border_color(theme.colors.method_get)
                          .bg(theme.colors.method_get.opacity(0.12))
                          .text_color(theme.colors.method_get)
                    })
                    .when(add_kind != TrpcProcKind::Query, |el| {
                        el.border_color(theme.colors.border)
                          .bg(theme.colors.bg_secondary)
                          .text_color(theme.colors.text_muted)
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.trpc_pg_add_kind = TrpcProcKind::Query; cx.notify();
                    }))
                    .child("Q")
            )
            // M chip
            .child(
                div()
                    .id("trpc-pg-kind-m")
                    .h(px(24.0))
                    .w(px(22.0))
                    .flex().items_center().justify_center()
                    .cursor_pointer()
                    .border_1()
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .when(add_kind == TrpcProcKind::Mutation, |el| {
                        el.border_color(theme.colors.method_post)
                          .bg(theme.colors.method_post.opacity(0.12))
                          .text_color(theme.colors.method_post)
                    })
                    .when(add_kind != TrpcProcKind::Mutation, |el| {
                        el.border_color(theme.colors.border)
                          .bg(theme.colors.bg_secondary)
                          .text_color(theme.colors.text_muted)
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.trpc_pg_add_kind = TrpcProcKind::Mutation; cx.notify();
                    }))
                    .child("M")
            )
            // Name input
            .child(
                div()
                    .flex_1()
                    .h(px(26.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    .child(Input::new(&self.trpc_pg_add_input).bordered(false))
            )
            // Add button
            .child(
                div()
                    .id("trpc-pg-add-btn")
                    .h(px(26.0))
                    .px(px(8.0))
                    .flex().items_center().gap(px(4.0))
                    .bg(theme.colors.accent.opacity(0.08))
                    .border_1()
                    .border_color(theme.colors.accent.opacity(0.3))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.accent.opacity(0.16)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let name = this.trpc_pg_add_input.read(cx).value().trim().to_string();
                        if !name.is_empty() {
                            let kind = this.trpc_pg_add_kind;
                            this.trpc_pg_procedures.push(TrpcPlaygroundProc { kind, name });
                            this.queue_editor(PendingEditor::TrpcPgAddInput, String::new());
                            cx.notify();
                        }
                    }))
                    .child(icon(ICON_PLUS, ICON_SM, theme.colors.accent))
            )
    }
}
