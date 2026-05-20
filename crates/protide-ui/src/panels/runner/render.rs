use gpui::{Context, IntoElement, MouseButton, ParentElement, Render, Styled, Window, div, px, prelude::*};
use super::{RunnerPanel, RowStatus};
use crate::theme;

impl Render for RunnerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let running = self.running;
        let total = self.total;
        let current = self.current;
        let passed = self.passed();
        let failed = self.failed();
        let rows = self.rows.clone();

        div()
            .id("runner-panel")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_primary)
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .h(px(36.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.text_primary)
                            .child("Collection Runner"),
                    )
                    .when(running, |el| {
                        el.child(
                            div()
                                .id("runner-stop-btn")
                                .px_2()
                                .h(px(24.0))
                                .flex()
                                .items_center()
                                .rounded_sm()
                                .text_xs()
                                .text_color(theme.colors.status_client_error)
                                .bg(theme.colors.status_client_error.opacity(0.1))
                                .cursor_pointer()
                                .hover(|s| s.opacity(0.75))
                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    this.stop(cx);
                                }))
                                .child("Stop"),
                        )
                    }),
            )
            // Progress bar
            .when(total > 0, |el| {
                let pct = if total > 0 { (current + 1) as f32 / total as f32 } else { 0.0 };
                el.child(
                    div()
                        .w_full()
                        .h(px(3.0))
                        .bg(theme.colors.border)
                        .flex_shrink_0()
                        .child(
                            div()
                                .h_full()
                                .w(gpui::relative(pct))
                                .bg(theme.colors.accent),
                        ),
                )
            })
            // Rows
            .child(
                div()
                    .id("runner-rows")
                    .flex_1()
                    .overflow_scroll()
                    .flex()
                    .flex_col()
                    .p_2()
                    .gap_px()
                    .children(rows.iter().map(|row| {
                        let (icon, color) = match &row.status {
                            RowStatus::Pending  => ("·", theme.colors.text_muted),
                            RowStatus::Running  => ("▶", theme.colors.accent),
                            RowStatus::Passed   => ("✓", theme.colors.status_success),
                            RowStatus::Failed(_)=> ("✗", theme.colors.status_client_error),
                        };
                        let error = if let RowStatus::Failed(e) = &row.status {
                            Some(e.clone())
                        } else {
                            None
                        };
                        let name = row.name.clone();

                        div()
                            .flex()
                            .flex_col()
                            .px_2()
                            .py(px(3.0))
                            .rounded_sm()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .w(px(14.0))
                                            .text_xs()
                                            .text_color(color)
                                            .child(icon),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_xs()
                                            .text_color(theme.colors.text_primary)
                                            .child(name),
                                    ),
                            )
                            .when_some(error, |el, err| {
                                el.child(
                                    div()
                                        .ml(px(18.0))
                                        .text_xs()
                                        .text_color(theme.colors.status_client_error)
                                        .child(err),
                                )
                            })
                    }))
                    .when(rows.is_empty() && !running, |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h(px(80.0))
                                .text_xs()
                                .text_color(theme.colors.text_muted)
                                .child("No requests run yet"),
                        )
                    }),
            )
            // Summary bar
            .when(!rows.is_empty(), |el| {
                el.child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_3()
                        .h(px(28.0))
                        .border_t_1()
                        .border_color(theme.colors.border)
                        .bg(theme.colors.bg_secondary)
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.colors.status_success)
                                .child(format!("{passed} passed")),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(if failed > 0 {
                                    theme.colors.status_client_error
                                } else {
                                    theme.colors.text_muted
                                })
                                .child(format!("{failed} failed")),
                        )
                        .when(running, |el| {
                            el.child(
                                div()
                                    .text_xs()
                                    .text_color(theme.colors.text_muted)
                                    .child(format!("{}/{total} done", current + 1)),
                            )
                        }),
                )
            })
    }
}
