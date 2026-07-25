use super::*;

impl ResponsePanel {
    pub(super) fn render_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let tabs = ["Body", "Headers", "Cookies", "Tests", "Extract"];
        let active_tab = self.active_tab;
        let test_count = self.test_results.len();
        let passed_count = self.test_results.iter().filter(|t| t.passed).count();

        div()
            .id("response-tabs")
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .px(px(16.0))
            .gap(px(0.0))
            .overflow_scroll()
            .border_b_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_primary)
            .children(tabs.iter().enumerate().map(|(i, tab)| {
                let is_active = i == active_tab;
                let is_tests_tab = i == 3;
                let show_badge = is_tests_tab && test_count > 0;
                let all_passed = passed_count == test_count;

                div()
                    .flex()
                    .items_center()
                    .h_full()
                    .child(
                        div()
                            .id(SharedString::from(format!("response-tab-{}", i)))
                            .px(px(12.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_secondary.opacity(0.3)))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(if is_active {
                                        gpui::FontWeight::MEDIUM
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .text_color(if is_active {
                                        theme.colors.text_primary
                                    } else {
                                        theme.colors.text_secondary
                                    })
                                    .child(*tab)
                            )
                            .when(show_badge, |el| {
                                el.child(
                                    div()
                                        .px(px(5.0))
                                        .py(px(1.0))
                                        .bg(if is_active {
                                            if all_passed {
                                                theme.colors.status_success.opacity(0.15)
                                            } else {
                                                theme.colors.status_client_error.opacity(0.15)
                                            }
                                        } else {
                                            theme.colors.bg_tertiary
                                        })
                                        .text_size(px(10.0))
                                        .text_color(if is_active {
                                            if all_passed {
                                                theme.colors.status_success
                                            } else {
                                                theme.colors.status_client_error
                                            }
                                        } else {
                                            theme.colors.text_muted
                                        })
                                        .child(format!("{}/{}", passed_count, test_count))
                                )
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_tab(i, cx);
                            }))
                    )
                    .when(i < tabs.len() - 1, |el| {
                        el.child(
                            div()
                                .h(px(16.0))
                                .w(px(1.0))
                                .bg(theme.colors.border)
                        )
                    })
            }))
    }
}
