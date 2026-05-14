use super::*;

impl ResponsePanel {
    pub(super) fn render_tests_tab(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let test_count = self.test_results.len();
        let passed_count = self.test_results.iter().filter(|t| t.passed).count();
        let failed_count = test_count - passed_count;

        if test_count == 0 {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(icon(ICON_CHECK, ICON_MD, theme.colors.text_muted.opacity(0.5)))
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("No tests have been run yet")
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted.opacity(0.7))
                                .child("Add tests in the Scripts tab and send a request")
                        )
                )
                .into_any_element();
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            // Summary bar
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .pb(px(12.0))
                    .child(
                        div()
                            .px(px(10.0))
                            .py(px(6.0))
                            .bg(theme.colors.status_success.opacity(0.12))
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success))
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.status_success)
                                    .child(format!("{} passed", passed_count))
                            )
                    )
                    .when(failed_count > 0, |el| {
                        el.child(
                            div()
                                .px(px(10.0))
                                .py(px(6.0))
                                .bg(theme.colors.status_client_error.opacity(0.12))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(icon(ICON_CLOSE, ICON_SM, theme.colors.status_client_error))
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme.colors.status_client_error)
                                        .child(format!("{} failed", failed_count))
                                )
                        )
                    })
            )
            // Test results list
            .child(
                div()
                    .id("tests-list")
                    .flex_1()
                    .w_full()
                    .overflow_scroll()
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_tertiary)
                    .children(self.test_results.iter().enumerate().map(|(i, result)| {
                        let is_last = i == test_count - 1;
                        div()
                            .w_full()
                            .px(px(12.0))
                            .py(px(10.0))
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .when(!is_last, |el| {
                                el.border_b_1()
                                  .border_color(theme.colors.border.opacity(0.5))
                            })
                            // Status icon
                            .child(
                                div()
                                    .size(px(20.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(result.passed, |el| {
                                        el.bg(theme.colors.status_success.opacity(0.15))
                                          .child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success))
                                    })
                                    .when(!result.passed, |el| {
                                        el.bg(theme.colors.status_client_error.opacity(0.15))
                                          .child(icon(ICON_CLOSE, ICON_SM, theme.colors.status_client_error))
                                    })
                            )
                            // Test name
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.text_primary)
                                            .child(result.name.clone())
                                    )
                                    .when(!result.passed && !result.expected.is_empty(), |el| {
                                        el.child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(8.0))
                                                .child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .text_color(theme.colors.text_muted)
                                                        .child(format!("Expected: {}", result.expected))
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .text_color(theme.colors.text_muted)
                                                        .child(format!("Actual: {}", result.actual))
                                                )
                                        )
                                    })
                            )
                    }))
            )
            .into_any_element()
    }
}
