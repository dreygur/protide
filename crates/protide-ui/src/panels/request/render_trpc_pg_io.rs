//! tRPC Playground — middle (params) and right (response) pane rendering

use gpui::{
    div, prelude::*, px, Context, IntoElement, ParentElement, Styled,
};
use gpui_component::input::Input;
use crate::theme;
use crate::components::icons::{icon, ICON_SM, ICON_MD, ICON_PLAY, ICON_LOADER, ICON_COPY};
use super::super::request_types::TrpcProcKind;
use protide_core::execution::ws::WebSocketExecutor;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_pg_middle(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .min_w(px(200.0))
            // Header with selected procedure badge
            .child({
                let selected = self.trpc.pg_selected.and_then(|i| self.trpc.pg_procedures.get(i));
                let theme2 = theme.clone();
                let badge = selected.map(|p| {
                    let (col, lbl) = match p.kind {
                        TrpcProcKind::Query => (theme2.colors.method_get, "Q"),
                        TrpcProcKind::Mutation => (theme2.colors.method_post, "M"),
                    };
                    (col, lbl, p.name.clone())
                });

                div()
                    .h(px(36.0))
                    .flex_none()
                    .w_full()
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_muted)
                            .child("Parameters")
                    )
                    .when_some(badge, |el, (col, lbl, name)| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .child(
                                    div()
                                        .px(px(5.0))
                                        .py(px(1.0))
                                        .bg(col.opacity(0.12))
                                        .text_size(px(10.0))
                                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                        .text_color(col)
                                        .child(lbl)
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_family("JetBrains Mono")
                                        .text_color(theme.colors.text_secondary)
                                        .child(name)
                                )
                        )
                    })
            })
            // JSON params editor
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(Input::new(&self.trpc.params_editor).appearance(false).h_full())
            )
            // Run bar
            .child(self.render_pg_run_bar(cx))
            .into_any_element()
    }

    fn render_pg_run_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let loading = self.trpc.pg_loading;
        let has_sel = self.trpc.pg_selected.is_some();
        let active = has_sel && !loading;

        div()
            .h(px(44.0))
            .flex_none()
            .w_full()
            .px(px(12.0))
            .flex()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_secondary)
            .child(
                div()
                    .id("trpc-pg-run-btn")
                    .h(px(30.0))
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .border_1()
                    .when(active, |el| {
                        el.bg(theme.colors.accent.opacity(0.1))
                          .border_color(theme.colors.accent.opacity(0.4))
                          .cursor_pointer()
                          .hover(|s| s.bg(theme.colors.accent.opacity(0.2)))
                    })
                    .when(!active, |el| {
                        el.bg(theme.colors.bg_tertiary)
                          .border_color(theme.colors.border)
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.trpc.pg_selected.is_some() && !this.trpc.pg_loading {
                            this.run_trpc_playground(cx);
                        }
                    }))
                    .child(if loading {
                        icon(ICON_LOADER, ICON_SM, theme.colors.accent)
                    } else {
                        icon(ICON_PLAY, ICON_SM, if active { theme.colors.accent } else { theme.colors.text_muted })
                    })
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if active { theme.colors.accent } else { theme.colors.text_muted })
                            .child(if loading { "Running…" } else { "Run" })
                    )
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(theme.colors.text_muted.opacity(0.6))
                    .child("tRPC v10")
            )
    }

    pub(super) fn render_pg_right(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let status = self.trpc.pg_status;
        let elapsed = self.trpc.pg_elapsed;
        let is_error = self.trpc.pg_error.is_some();
        let has_result = self.trpc.pg_response.is_some() || is_error;

        let status_color = match status {
            Some(s) if s < 300 => theme.colors.status_success,
            Some(s) if s < 400 => theme.colors.status_redirect,
            Some(s) if s < 500 => theme.colors.status_client_error,
            Some(_) => theme.colors.status_server_error,
            None => theme.colors.text_muted,
        };
        let resp_color = if is_error { theme.colors.status_client_error } else { status_color };

        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .min_w(px(200.0))
            // Header: status badge + timing + copy
            .child(
                div()
                    .h(px(36.0))
                    .flex_none()
                    .w_full()
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_muted)
                            .child("Response")
                    )
                    .when_some(status.filter(|_| has_result), |el, s| {
                        el.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .bg(resp_color.opacity(0.12))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(resp_color)
                                .child(s.to_string())
                        )
                    })
                    .when_some(elapsed.filter(|_| has_result), |el, d| {
                        el.child(
                            div()
                                .text_size(px(10.0))
                                .text_color(theme.colors.text_muted)
                                .child(format!("{}ms", d.as_millis()))
                        )
                    })
                    .when(has_result, |el| {
                        el.child(
                            div()
                                .id("trpc-pg-copy-resp")
                                .ml_auto()
                                .h(px(24.0))
                                .px(px(8.0))
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .border_1()
                                .border_color(theme.colors.border)
                                .text_size(px(10.0))
                                .text_color(theme.colors.text_secondary)
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.bg_elevated))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let content = this.trpc.pg_result_viewer.read(cx).value().to_string();
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(content));
                                }))
                                .child(icon(ICON_COPY, ICON_MD, theme.colors.text_muted))
                                .child("Copy")
                        )
                    })
            )
            // Result viewer (read-only syntax-highlighted)
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(Input::new(&self.trpc.pg_result_viewer).disabled(true).appearance(false).h_full())
            )
            .into_any_element()
    }
}
