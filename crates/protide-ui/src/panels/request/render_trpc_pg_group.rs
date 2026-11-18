use super::super::request_types::PendingEditor;
use super::RequestPanel;
use crate::components::icons::{ICON_CHECK, ICON_CHEVRON_DOWN, ICON_SM, icon};
use crate::theme;
use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*, px};
use gpui_component::{Sizable, input::Input};
use protide_core::execution::ws::WebSocketExecutor;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_pg_group_header(
        &self,
        router: String,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let is_editing_group = self.trpc_pg_editing_group.as_deref() == Some(router.as_str());
        let router_click = router.clone();
        let router_ok = router.clone();
        if is_editing_group {
            div()
                .h(px(22.0))
                .px(px(6.0))
                .flex()
                .items_center()
                .gap(px(4.0))
                .bg(theme.colors.bg_tertiary)
                .border_b_1()
                .border_color(theme.colors.accent.opacity(0.5))
                .child(
                    div().flex_1().h_full().overflow_hidden().child(
                        Input::new(&self.trpc_pg_group_edit_input)
                            .bordered(false)
                            .with_size(gpui_component::Size::XSmall),
                    ),
                )
                .child(
                    div()
                        .id(format!("trpc-grp-ok-{}", router))
                        .w(px(20.0))
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                let nr = this
                                    .trpc_pg_group_edit_input
                                    .read(cx)
                                    .value()
                                    .trim()
                                    .to_string();
                                let old = router_ok.clone();
                                for proc in &mut this.trpc_pg_procedures {
                                    let or = proc.router().to_string();
                                    let lf = proc.leaf().to_string();
                                    let hit = if old == "(root)" {
                                        or.is_empty()
                                    } else {
                                        or == old
                                    };
                                    if hit {
                                        proc.name = if nr.is_empty() {
                                            lf
                                        } else {
                                            format!("{}.{}", nr, lf)
                                        };
                                    }
                                }
                                this.trpc_pg_editing_group = None;
                                cx.notify();
                            }),
                        )
                        .child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success)),
                )
                .child(
                    div()
                        .id(format!("trpc-grp-x-{}", router))
                        .w(px(20.0))
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .hover(|s| s.text_color(theme.colors.status_client_error))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.trpc_pg_editing_group = None;
                                cx.notify();
                            }),
                        )
                        .child("×"),
                )
                .into_any_element()
        } else {
            div()
                .id(format!("trpc-grp-hdr-{}", router))
                .h(px(22.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(5.0))
                .bg(theme.colors.bg_primary)
                .border_b_1()
                .border_color(theme.colors.border)
                .cursor_pointer()
                .hover(|s| s.bg(theme.colors.hover_overlay))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let d = if router_click == "(root)" {
                        String::new()
                    } else {
                        router_click.clone()
                    };
                    this.trpc_pg_editing_group = Some(router_click.clone());
                    this.queue_editor(PendingEditor::TrpcPgGroupEditInput, d);
                    cx.notify();
                }))
                .child(icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted))
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text_muted)
                        .font_family("JetBrains Mono")
                        .child(router.clone()),
                )
                .into_any_element()
        }
    }
}
