use super::*;

impl ResponsePanel {
    pub(super) fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.0))
            .border_b_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_secondary)
            // Title with icon
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(icon(ICON_ARROW_DOWN, ICON_SM, theme.colors.text_muted))
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Response")
                    )
            )
            // Status info
            .child(self.render_status_info(cx))
    }

    pub(super) fn render_status_info(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        if self.loading {
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .size(px(14.0))

                        .border_2()
                        .border_color(theme.colors.accent.opacity(0.5))
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child("Sending...")
                )
        } else if let Some(error) = &self.error {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(10.0))
                .py(px(4.0))
                .bg(theme.colors.status_client_error.opacity(0.1))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .child(icon(ICON_CLOSE, ICON_SM, theme.colors.status_client_error))
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.colors.status_client_error)
                        .child(truncate_error(error))
                )
        } else if let Some(response) = &self.response {
            let status_color = theme.status_color(response.status);
            let time_color = if response.status == 0 {
                theme.colors.text_muted
            } else {
                theme.status_color(response.status)
            };
            let description = status_description(response.status);

            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                // Status badge with description
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        // Status code badge
                        .child(
                            div()
                                .px(px(10.0))
                                .py(px(4.0))
                                .bg(status_color.opacity(0.12))
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(status_color)
                                        .child(format!("{} {}", response.status, response.status_text))
                                )
                        )
                        // Description text (if available)
                        .when_some(description, |el, desc| {
                            el.child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(format!("· {}", desc))
                            )
                        })
                )
                // Time
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .text_size(px(11.0))
                        .text_color(time_color)
                        .child(format!("{}ms", response.time.as_millis()))
                )
                // Size
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .text_size(px(11.0))
                        .text_color(theme.colors.text_muted)
                        .child(format_size(response.size))
                )
        } else {
            div()
        }
    }

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
