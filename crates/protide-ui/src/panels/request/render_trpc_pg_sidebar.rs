//! tRPC Playground — procedure list and add-procedure row rendering

use gpui::{
    div, prelude::*, px, Context, IntoElement, ParentElement, Styled,
};
use gpui_component::{input::Input, Sizable};
use crate::theme;
use crate::components::icons::{icon, ICON_SM, ICON_CHECK, ICON_CHEVRON_DOWN, ICON_EDIT, ICON_PLUS};
use super::super::request_types::{TrpcProcKind, TrpcPlaygroundProc, PendingEditor};
use protide_core::execution::ws::WebSocketExecutor;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_pg_proc_list(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let search = self.trpc.pg_search_input.read(cx).value().to_string();
        let q = search.to_lowercase();

        let filtered: Vec<usize> = self.trpc.pg_procedures.iter().enumerate()
            .filter(|(_, p)| q.is_empty() || p.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();

        let mut routers: Vec<String> = Vec::new();
        for &idx in &filtered {
            let r = self.trpc.pg_procedures[idx].router();
            let key = if r.is_empty() { "(root)".to_string() } else { r.to_string() };
            if !routers.contains(&key) { routers.push(key); }
        }

        let selected = self.trpc.pg_selected;
        let editing = self.trpc.pg_editing;
        let edit_kind = self.trpc.pg_edit_kind;

        div()
            .id("trpc-pg-proc-list")
            .flex_1()
            .w_full()
            .overflow_scroll()
            .flex()
            .flex_col()
            .children(routers.into_iter().map(|router| {
                let procs: Vec<usize> = filtered.iter().copied()
                    .filter(|&i| {
                        let r = self.trpc.pg_procedures[i].router();
                        (if r.is_empty() { "(root)" } else { r }) == router.as_str()
                    })
                    .collect();

                let is_editing_group = self.trpc.pg_editing_group.as_deref() == Some(router.as_str());
                let router_click = router.clone();
                let router_ok = router.clone();

                // Group header — normal or edit mode
                let group_header: gpui::AnyElement = if is_editing_group {
                    div()
                        .h(px(22.0)).px(px(6.0)).flex().items_center().gap(px(4.0))
                        .bg(theme.colors.bg_tertiary).border_b_1()
                        .border_color(theme.colors.accent.opacity(0.5))
                        .child(div().flex_1().h_full().overflow_hidden()
                            .child(Input::new(&self.trpc.pg_group_edit_input)
                                .bordered(false).with_size(gpui_component::Size::XSmall)))
                        .child(div()
                            .id(format!("trpc-grp-ok-{}", router))
                            .w(px(20.0)).h_full().flex().items_center().justify_center()
                            .cursor_pointer()
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                let nr = this.trpc.pg_group_edit_input.read(cx).value().trim().to_string();
                                let old = router_ok.clone();
                                for proc in &mut this.trpc.pg_procedures {
                                    let or = proc.router().to_string();
                                    let lf = proc.leaf().to_string();
                                    let hit = if old == "(root)" { or.is_empty() } else { or == old };
                                    if hit {
                                        proc.name = if nr.is_empty() { lf } else { format!("{}.{}", nr, lf) };
                                    }
                                }
                                this.trpc.pg_editing_group = None;
                                cx.notify();
                            }))
                            .child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success)))
                        .child(div()
                            .id(format!("trpc-grp-x-{}", router))
                            .w(px(20.0)).h_full().flex().items_center().justify_center()
                            .cursor_pointer()
                            .text_size(px(12.0)).text_color(theme.colors.text_muted)
                            .hover(|s| s.text_color(theme.colors.status_client_error))
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.trpc.pg_editing_group = None; cx.notify();
                            }))
                            .child("×"))
                        .into_any_element()
                } else {
                    div()
                        .id(format!("trpc-grp-hdr-{}", router))
                        .h(px(22.0)).px(px(10.0)).flex().items_center().gap(px(5.0))
                        .bg(theme.colors.bg_primary).border_b_1().border_color(theme.colors.border)
                        .cursor_pointer().hover(|s| s.bg(theme.colors.hover_overlay))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let d = if router_click == "(root)" { String::new() } else { router_click.clone() };
                            this.trpc.pg_editing_group = Some(router_click.clone());
                            this.queue_editor(PendingEditor::TrpcPgGroupEditInput, d);
                            cx.notify();
                        }))
                        .child(icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted))
                        .child(div()
                            .text_size(px(10.0)).font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_muted).font_family("JetBrains Mono")
                            .child(router.clone()))
                        .into_any_element()
                };

                div()
                    .flex_none().flex().flex_col()
                    .child(group_header)
                    .children(procs.into_iter().map(|idx| {
                        let kind = self.trpc.pg_procedures[idx].kind;
                        let is_sel = selected == Some(idx);
                        let is_editing_row = editing == Some(idx);
                        let kc = match kind {
                            TrpcProcKind::Query    => theme.colors.method_get,
                            TrpcProcKind::Mutation => theme.colors.method_post,
                        };

                        if is_editing_row {
                            div()
                                .id(format!("trpc-pg-row-{}", idx))
                                .h(px(30.0)).pl(px(8.0)).pr(px(8.0))
                                .flex().items_center().gap(px(4.0))
                                .border_b_1().border_color(theme.colors.border.opacity(0.5))
                                .bg(theme.colors.accent.opacity(0.04))
                                // Q toggle
                                .child(div()
                                    .id(format!("trpc-pg-ek-q-{}", idx))
                                    .w(px(16.0)).h(px(16.0)).flex_none().flex().items_center().justify_center()
                                    .bg(if edit_kind == TrpcProcKind::Query { theme.colors.method_get.opacity(0.12) } else { theme.colors.bg_secondary })
                                    .text_size(px(9.0)).font_weight(gpui::FontWeight::EXTRA_BOLD)
                                    .text_color(if edit_kind == TrpcProcKind::Query { theme.colors.method_get } else { theme.colors.text_muted })
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| { this.trpc.pg_edit_kind = TrpcProcKind::Query; cx.notify(); }))
                                    .child("Q"))
                                // M toggle
                                .child(div()
                                    .id(format!("trpc-pg-ek-m-{}", idx))
                                    .w(px(16.0)).h(px(16.0)).flex_none().flex().items_center().justify_center()
                                    .bg(if edit_kind == TrpcProcKind::Mutation { theme.colors.method_post.opacity(0.12) } else { theme.colors.bg_secondary })
                                    .text_size(px(9.0)).font_weight(gpui::FontWeight::EXTRA_BOLD)
                                    .text_color(if edit_kind == TrpcProcKind::Mutation { theme.colors.method_post } else { theme.colors.text_muted })
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| { this.trpc.pg_edit_kind = TrpcProcKind::Mutation; cx.notify(); }))
                                    .child("M"))
                                // Name input
                                .child(div().flex_1().h_full().overflow_hidden()
                                    .child(Input::new(&self.trpc.pg_edit_input)
                                        .bordered(false).with_size(gpui_component::Size::XSmall)))
                                // Confirm
                                .child(div()
                                    .id(format!("trpc-pg-ok-{}", idx))
                                    .size(px(16.0)).flex().items_center().justify_center().cursor_pointer()
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                        let n = this.trpc.pg_edit_input.read(cx).value().trim().to_string();
                                        if !n.is_empty() {
                                            if let Some(proc) = this.trpc.pg_procedures.get_mut(idx) {
                                                proc.name = n;
                                                proc.kind = this.trpc.pg_edit_kind;
                                            }
                                        }
                                        this.trpc.pg_editing = None;
                                        cx.notify();
                                    }))
                                    .child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success)))
                                // Cancel
                                .child(div()
                                    .id(format!("trpc-pg-cl-{}", idx))
                                    .size(px(14.0)).flex().items_center().justify_center()
                                    .text_size(px(12.0)).text_color(theme.colors.text_muted).cursor_pointer()
                                    .hover(|s| s.text_color(theme.colors.status_client_error))
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.trpc.pg_editing = None; cx.notify();
                                    }))
                                    .child("×"))
                                .into_any_element()
                        } else {
                            let leaf = self.trpc.pg_procedures.get(idx).map(|p| p.leaf().to_string()).unwrap_or_default();
                            div()
                                .id(format!("trpc-pg-row-{}", idx))
                                .h(px(30.0)).pl(px(22.0)).pr(px(8.0))
                                .flex().items_center().gap(px(6.0))
                                .cursor_pointer().border_b_1().border_color(theme.colors.border.opacity(0.5))
                                .when(is_sel, |el| el.bg(theme.colors.accent.opacity(0.08)).border_l_2().border_color(theme.colors.accent))
                                .when(!is_sel, |el| el.hover(|s| s.bg(theme.colors.hover_overlay)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(proc) = this.trpc.pg_procedures.get(idx) {
                                        this.trpc.pg_selected = Some(idx);
                                        this.trpc.procedure = proc.full_procedure();
                                    }
                                    cx.notify();
                                }))
                                // Kind badge
                                .child(div()
                                    .w(px(16.0)).h(px(16.0)).flex_none().flex().items_center().justify_center()
                                    .bg(kc.opacity(0.12)).text_size(px(9.0)).font_weight(gpui::FontWeight::EXTRA_BOLD)
                                    .text_color(kc)
                                    .child(match kind { TrpcProcKind::Query => "Q", TrpcProcKind::Mutation => "M" }))
                                // Leaf name
                                .child(div()
                                    .flex_1().text_size(px(12.0)).font_family("JetBrains Mono")
                                    .text_color(if is_sel { theme.colors.text_primary } else { theme.colors.text_secondary })
                                    .child(leaf))
                                // Edit button
                                .child(div()
                                    .id(format!("trpc-pg-ed-{}", idx))
                                    .size(px(14.0)).flex().items_center().justify_center()
                                    .text_color(theme.colors.text_muted.opacity(0.0))
                                    .hover(|s| s.text_color(theme.colors.accent))
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        if let Some(proc) = this.trpc.pg_procedures.get(idx) {
                                            let n = proc.name.clone();
                                            let k = proc.kind;
                                            this.trpc.pg_editing = Some(idx);
                                            this.trpc.pg_edit_kind = k;
                                            this.queue_editor(PendingEditor::TrpcPgEditInput, n);
                                            cx.notify();
                                        }
                                    }))
                                    .child(icon(ICON_EDIT, ICON_SM, theme.colors.text_muted)))
                                // Delete button
                                .child(div()
                                    .id(format!("trpc-pg-del-{}", idx))
                                    .size(px(14.0)).flex().items_center().justify_center()
                                    .text_size(px(12.0)).text_color(theme.colors.text_muted.opacity(0.0))
                                    .hover(|s| s.text_color(theme.colors.status_client_error))
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.trpc.pg_procedures.remove(idx);
                                        match this.trpc.pg_selected {
                                            Some(s) if s == idx => this.trpc.pg_selected = None,
                                            Some(s) if s > idx  => this.trpc.pg_selected = Some(s - 1),
                                            _ => {}
                                        }
                                        cx.notify();
                                    }))
                                    .child("×"))
                                .into_any_element()
                        }
                    }))
            }))
            .into_any_element()
    }

    pub(super) fn render_pg_add_row(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let add_kind = self.trpc.pg_add_kind;

        let chip = |id: &'static str, lbl: &'static str, active: bool, bc, bg, tc| {
            div()
                .id(id).h(px(24.0)).w(px(22.0))
                .flex().items_center().justify_center()
                .cursor_pointer().border_1()
                .text_size(px(10.0)).font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(active,  |el| el.border_color(bc).bg(bg).text_color(tc))
                .when(!active, |el| el.border_color(theme.colors.border).bg(theme.colors.bg_secondary).text_color(theme.colors.text_muted))
                .child(lbl)
        };
        div()
            .flex_none().h(px(40.0)).border_t_1().border_color(theme.colors.border)
            .bg(theme.colors.bg_primary).flex().items_center().gap(px(4.0)).px(px(6.0))
            .child(chip("trpc-pg-kind-q", "Q", add_kind == TrpcProcKind::Query,
                theme.colors.method_get, theme.colors.method_get.opacity(0.12), theme.colors.method_get)
                .on_click(cx.listener(|this, _, _, cx| { this.trpc.pg_add_kind = TrpcProcKind::Query; cx.notify(); })))
            .child(chip("trpc-pg-kind-m", "M", add_kind == TrpcProcKind::Mutation,
                theme.colors.method_post, theme.colors.method_post.opacity(0.12), theme.colors.method_post)
                .on_click(cx.listener(|this, _, _, cx| { this.trpc.pg_add_kind = TrpcProcKind::Mutation; cx.notify(); })))
            .child(div().flex_1().h(px(26.0)).border_1().border_color(theme.colors.border).overflow_hidden()
                .child(Input::new(&self.trpc.pg_add_input).bordered(false).with_size(gpui_component::Size::XSmall)))
            .child(div()
                .id("trpc-pg-add-btn").h(px(26.0)).px(px(8.0))
                .flex().items_center().gap(px(4.0))
                .bg(theme.colors.accent.opacity(0.08)).border_1().border_color(theme.colors.accent.opacity(0.3))
                .cursor_pointer().hover(|s| s.bg(theme.colors.accent.opacity(0.16)))
                .on_click(cx.listener(|this, _, _, cx| {
                    let name = this.trpc.pg_add_input.read(cx).value().trim().to_string();
                    if !name.is_empty() {
                        let kind = this.trpc.pg_add_kind;
                        this.trpc.pg_procedures.push(TrpcPlaygroundProc { kind, name });
                        this.queue_editor(PendingEditor::TrpcPgAddInput, String::new());
                        cx.notify();
                    }
                }))
                .child(icon(ICON_PLUS, ICON_SM, theme.colors.accent)))
            .into_any_element()
    }
}
