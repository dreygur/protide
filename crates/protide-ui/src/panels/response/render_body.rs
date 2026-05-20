use super::*;

impl ResponsePanel {
    pub(super) fn render_body_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        // Get Content-Type header for format label
        let content_type = response.headers.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.to_lowercase());

        // Determine format label and color
        let (format_label, format_color) = if let Some(ct) = &content_type {
            if ct.contains("application/json") || ct.contains("+json") {
                ("JSON", theme.colors.status_success)
            } else if ct.contains("text/html") {
                ("HTML", theme.colors.method_patch)
            } else if ct.contains("application/xml") || ct.contains("text/xml") || ct.contains("+xml") {
                ("XML", theme.colors.method_put)
            } else if ct.contains("text/css") {
                ("CSS", theme.colors.method_delete)
            } else if ct.contains("javascript") || ct.contains("text/js") {
                ("JS", theme.colors.accent)
            } else {
                ("Text", theme.colors.text_muted)
            }
        } else {
            // Detect from content
            let trimmed = response.body.trim();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                ("JSON", theme.colors.status_success)
            } else if trimmed.starts_with('<') {
                ("XML", theme.colors.method_put)
            } else {
                ("Text", theme.colors.text_muted)
            }
        };

        if response.body.trim().is_empty() {
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
                                .child(icon(ICON_CIRCLE_CHECK, ICON_MD, theme.colors.text_muted.opacity(0.5)))
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("Response body is empty")
                        )
                )
                .into_any_element();
        }

        let line_count = self.body_viewer.read(cx).content().lines().count();
        let search_active = self.search_active;
        let search_query = self.search_input.read(cx).get_text().to_string();
        let match_count = if search_active && !search_query.is_empty() {
            let body = self.body_viewer.read(cx).content().to_string();
            let q = search_query.to_lowercase();
            body.to_lowercase().matches(q.as_str()).count()
        } else {
            0
        };

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Toolbar row - relative so Copy button can anchor to right_0
            .child({
                let is_copied = self.copy_feedback == Some(CopyFeedback::Body);
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .relative()
                    // Left: Format badge + line count + search toggle
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .bg(format_color.opacity(0.12))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(format_color)
                                    .child(format_label)
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(format!("{} lines", line_count))
                            )
                            .child(
                                div()
                                    .id("resp-search-toggle")
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .text_size(px(10.0))
                                    .cursor_pointer()
                                    .border_1()
                                    .when(search_active, |el| {
                                        el.border_color(theme.colors.accent)
                                          .text_color(theme.colors.accent)
                                          .bg(theme.colors.accent.opacity(0.08))
                                    })
                                    .when(!search_active, |el| {
                                        el.border_color(theme.colors.border)
                                          .text_color(theme.colors.text_muted)
                                          .bg(theme.colors.bg_secondary)
                                    })
                                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.search_active = !this.search_active;
                                        cx.notify();
                                    }))
                                    .child("Search")
                            )
                    )
                    // Right: Copy button - deferred so it paints above the JSON tree below
                    .child(deferred(
                        div()
                            .id("copy-body-btn")
                            .absolute()
                            .right_0()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(10.0))
                            .py(px(5.0))
                            .text_size(px(11.0))
                            .when(is_copied, |el| el.text_color(theme.colors.status_success).border_color(theme.colors.status_success))
                            .when(!is_copied, |el| el.text_color(theme.colors.text_secondary).border_color(theme.colors.border))
                            .cursor_pointer()
                            .border_1()
                            .bg(theme.colors.bg_primary)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let content = this.body_viewer.read(cx).content().to_string();
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(content));
                                this.show_copy_feedback(CopyFeedback::Body, cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .when(is_copied, |el| el.child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success)))
                                    .when(!is_copied, |el| el.child(icon(ICON_COPY, ICON_MD, theme.colors.text_secondary)))
                            )
                            .child(if is_copied { "Copied!" } else { "Copy" })
                    ).with_priority(1))
            })
            // Search bar row
            .when(search_active, |el| {
                el.child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .h(px(28.0))
                                .border_1()
                                .border_color(theme.colors.border)
                                .bg(theme.colors.bg_secondary)
                                .child(self.search_input.clone())
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .min_w(px(60.0))
                                .text_color(if match_count > 0 {
                                    theme.colors.status_success
                                } else if search_query.is_empty() {
                                    theme.colors.text_muted
                                } else {
                                    theme.colors.status_client_error
                                })
                                .child(if search_query.is_empty() {
                                    "0 matches".to_string()
                                } else {
                                    format!("{} match{}", match_count, if match_count == 1 { "" } else { "es" })
                                })
                        )
                        .child(
                            div()
                                .id("resp-search-close")
                                .px(px(6.0))
                                .py(px(2.0))
                                .text_size(px(10.0))
                                .cursor_pointer()
                                .text_color(theme.colors.text_muted)
                                .hover(|s| s.text_color(theme.colors.text_primary))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.search_active = false;
                                    cx.notify();
                                }))
                                .child("✕")
                        )
                )
            })
            // Body content: JSON tree (uniform_list) if parseable, else CodeEditor
            .child(
                if self.json_value.is_some() {
                    self.render_json_tree(cx)
                } else {
                    div()
                        .flex_1()
                        .w_full()
                        .overflow_hidden()
                        .child(self.body_viewer.clone())
                        .into_any_element()
                }
            )
            .into_any_element()
    }
}
