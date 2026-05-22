use super::*;

impl ResponsePanel {
    pub(super) fn render_extract_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        // Check if body is JSON
        let is_json = response.body.trim().starts_with('{') || response.body.trim().starts_with('[');

        if !is_json {
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
                                .text_size(px(24.0))
                                .text_color(theme.colors.text_muted.opacity(0.5))
                                .child("$")
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("JSONPath extraction requires JSON response")
                        )
                )
                .into_any_element();
        }

        let jsonpath_value = self.jsonpath_input.read(cx).value().to_string();

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            // Input row
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    // JSONPath label
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.colors.text_secondary)
                            .child("JSONPath:")
                    )
                    // Input field
                    .child(
                        div()
                            .flex_1()
                            .h(px(32.0))
                            .child(Input::new(&self.jsonpath_input))
                    )
                    // Extract button
                    .child(
                        div()
                            .id("extract-btn")
                            .h(px(32.0))
                            .px(px(12.0))
                            .flex()
                            .items_center()
                            .bg(theme.colors.accent)
                            .text_size(px(12.0))
                            .text_color(theme.colors.bg_primary)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.accent.opacity(0.85)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.run_extraction(window, cx);
                            }))
                            .child("Extract")
                    )
            )
            // Quick patterns
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Quick:")
                    )
                    .children(
                        [("$.data", "$.data"), ("$[0]", "$[0]"), ("$.id", "$.id"), ("$.token", "$.token")]
                            .into_iter()
                            .map(|(label, pattern)| {
                                let pattern = pattern.to_string();
                                div()
                                    .id(SharedString::from(format!("pattern-{}", label)))
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .bg(theme.colors.bg_tertiary)
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .text_size(px(10.0))
                                    .font_family("JetBrains Mono")
                                    .text_color(theme.colors.text_secondary)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.bg_elevated))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.jsonpath_input.update(cx, |input, cx| {
                                            input.set_value(pattern.clone(), window, cx);
                                        });
                                        this.run_extraction(window, cx);
                                    }))
                                    .child(label)
                            })
                    )
            )
            // Result display
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_tertiary)
                    .overflow_hidden()
                    .child(
                        match &self.extraction_result {
                            Some(Ok(value)) => {
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    // Success header
                                    .child(
                                        div()
                                            .w_full()
                                            .px(px(12.0))
                                            .py(px(8.0))
                                            .border_b_1()
                                            .border_color(theme.colors.border)
                                            .bg(theme.colors.status_success.opacity(0.1))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
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
                                                            .text_size(px(11.0))
                                                            .text_color(theme.colors.status_success)
                                                            .child(format!("Extracted: {}", jsonpath_value))
                                                    )
                                            )
                                            .child(
                                                div()
                                                    .id("copy-extract-btn")
                                                    .px(px(8.0))
                                                    .py(px(4.0))
                                                    .border_1()
                                                    .border_color(theme.colors.border)
                                                    .text_size(px(10.0))
                                                    .text_color(theme.colors.text_secondary)
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(theme.colors.bg_elevated))
                                                    .on_click({
                                                        let value = value.clone();
                                                        cx.listener(move |_this, _, _, cx| {
                                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.clone()));
                                                        })
                                                    })
                                                    .child("Copy")
                                            )
                                    )
                                    // Value display with syntax highlighting
                                    .child(
                                        div()
                                            .id("extract-value")
                                            .flex_1()
                                            .overflow_hidden()
                                            .child(Input::new(&self.extraction_editor).disabled(true).appearance(false).h_full())
                                    )
                                    .into_any_element()
                            }
                            Some(Err(error)) => {
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    // Error header
                                    .child(
                                        div()
                                            .w_full()
                                            .px(px(12.0))
                                            .py(px(8.0))
                                            .border_b_1()
                                            .border_color(theme.colors.border)
                                            .bg(theme.colors.status_client_error.opacity(0.1))
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
                                                    .text_size(px(11.0))
                                                    .text_color(theme.colors.status_client_error)
                                                    .child("Extraction failed")
                                            )
                                    )
                                    // Error message
                                    .child(
                                        div()
                                            .flex_1()
                                            .p(px(12.0))
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(theme.colors.text_muted)
                                                    .child(error.clone())
                                            )
                                    )
                                    .into_any_element()
                            }
                            None => {
                                div()
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
                                                    .text_size(px(14.0))
                                                    .text_color(theme.colors.text_muted.opacity(0.5))
                                                    .child("$")
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(theme.colors.text_muted)
                                                    .child("Enter a JSONPath expression and click Extract")
                                            )
                                    )
                                    .into_any_element()
                            }
                        }
                    )
            )
            .into_any_element()
    }
}
