use super::*;

impl ResponsePanel {
    pub(super) fn render_content(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        if self.loading {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(12.0))
                .child(
                    div()
                        .size(px(32.0))

                        .border_3()
                        .border_color(theme.colors.accent.opacity(0.4))
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.colors.text_muted)
                        .child("Sending request...")
                )
                .into_any_element();
        }

        if let Some(error) = &self.error {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(12.0))
                .child(
                    div()
                        .size(px(48.0))

                        .bg(theme.colors.status_client_error.opacity(0.1))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_size(px(24.0))
                                .text_color(theme.colors.status_client_error)
                                .child("!")
                        )
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.colors.text_primary)
                        .child("Request Failed")
                )
                .child(
                    div()
                        .max_w(px(400.0))
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child(error.clone())
                )
                .into_any_element();
        }

        // Tests tab can be shown even without a response (but will be empty)
        if self.active_tab == 3 {
            return div()
                .id("tests-scroll")
                .flex_1()
                .w_full()
                .overflow_scroll()
                .track_scroll(&self.content_scroll_handle)
                .child(self.render_tests_tab(cx))
                .vertical_scrollbar(&self.content_scroll_handle)
                .into_any_element();
        }

        if let Some(response) = &self.response {
            match self.active_tab {
                // Body tab handles its own scroll internally (editor + JSON tree)
                0 => self.render_body_tab(response, cx),
                // All other tabs are scrollable lists
                1 => div()
                    .id("headers-scroll")
                    .flex_1()
                    .w_full()
                    .overflow_scroll()
                    .track_scroll(&self.content_scroll_handle)
                    .child(self.render_headers_tab(response, cx))
                    .vertical_scrollbar(&self.content_scroll_handle)
                    .into_any_element(),
                2 => div()
                    .id("cookies-scroll")
                    .flex_1()
                    .w_full()
                    .overflow_scroll()
                    .track_scroll(&self.content_scroll_handle)
                    .child(self.render_cookies_tab(response, cx))
                    .vertical_scrollbar(&self.content_scroll_handle)
                    .into_any_element(),
                4 => div()
                    .id("extract-scroll")
                    .flex_1()
                    .w_full()
                    .overflow_scroll()
                    .track_scroll(&self.content_scroll_handle)
                    .child(self.render_extract_tab(response, cx))
                    .vertical_scrollbar(&self.content_scroll_handle)
                    .into_any_element(),
                _ => div().into_any_element(),
            }
        } else {
            // Empty state - engaging and helpful
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(16.0))
                // Illustration container
                .child(
                    div()
                        .size(px(80.0))
                        .bg(theme.colors.bg_tertiary)
                        .border_1()
                        .border_color(theme.colors.border.opacity(0.5))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(4.0))
                                // Animated-style arrows
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(icon(ICON_ARROW_DOWN, ICON_SM, theme.colors.text_muted.opacity(0.4)))
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(icon(ICON_ARROW_DOWN, ICON_MD, theme.colors.text_muted.opacity(0.6)))
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(icon(ICON_ARROW_DOWN, ICON_MD, theme.colors.text_muted))
                                )
                        )
                )
                // Title
                .child(
                    div()
                        .text_size(px(15.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text_primary)
                        .child("Ready to receive")
                )
                // Description
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child("Send a request to see the response here")
                )
                // Keyboard shortcut hint
                .child(
                    div()
                        .mt(px(8.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted.opacity(0.7))
                                .child("Press")
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(3.0))
                                        .bg(theme.colors.bg_elevated)
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .text_size(px(10.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme.colors.text_secondary)
                                        .child("⌘")
                                )
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(3.0))
                                        .bg(theme.colors.bg_elevated)
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .text_size(px(10.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme.colors.text_secondary)
                                        .child("↵")
                                )
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted.opacity(0.7))
                                .child("to send")
                        )
                )
                .into_any_element()
        }
    }
}
