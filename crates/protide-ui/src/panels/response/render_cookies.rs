use super::*;

impl ResponsePanel {
    pub(super) fn render_cookies_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        // Parse cookies from Set-Cookie headers
        let cookies: Vec<ParsedCookie> = response
            .headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
            .filter_map(|(_, v)| ParsedCookie::parse(v))
            .collect();

        let cookie_count = cookies.len();

        if cookie_count == 0 {
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
                                .child(icon(ICON_GLOBE, ICON_MD, theme.colors.text_muted.opacity(0.5)))
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("No cookies in response")
                        )
                )
                .into_any_element();
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            // Toolbar
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pb(px(12.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .bg(theme.colors.accent.opacity(0.12))
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.accent)
                                    .child(format!("{} cookies", cookie_count))
                            )
                    )
            )
            // Cookie table
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_tertiary)
                    .overflow_hidden()
                    // Header row
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .bg(theme.colors.bg_secondary)
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .child(
                                div()
                                    .w(px(self.cookie_col1_w))
                                    .min_w(px(60.0))
                                    .px(px(12.0))
                                    .py(px(10.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("NAME")
                            )
                            .child(self.render_col_drag_handle(1, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.0))
                                    .py(px(10.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("VALUE")
                            )
                            .child(self.render_col_drag_handle(2, cx))
                            .child(
                                div()
                                    .w(px(self.cookie_col3_w))
                                    .min_w(px(60.0))
                                    .px(px(12.0))
                                    .py(px(10.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("PATH")
                            )
                            .child(self.render_col_drag_handle(3, cx))
                            .child(
                                div()
                                    .w(px(self.cookie_col4_w))
                                    .min_w(px(60.0))
                                    .px(px(12.0))
                                    .py(px(10.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("FLAGS")
                            )
                    )
                    // Cookie rows
                    .children(cookies.into_iter().enumerate().map(|(i, cookie)| {
                        let has_border = i > 0;
                        let mut flags = Vec::new();
                        if cookie.secure { flags.push("Secure"); }
                        if cookie.http_only { flags.push("HttpOnly"); }
                        let flags_str = flags.join(", ");
                        let col1 = self.cookie_col1_w;
                        let col3 = self.cookie_col3_w;
                        let col4 = self.cookie_col4_w;

                        div()
                            .w_full()
                            .flex()
                            .when(has_border, |el| el.border_t_1().border_color(theme.colors.border.opacity(0.5)))
                            .hover(|s| s.bg(theme.colors.bg_secondary.opacity(0.3)))
                            .child(
                                div()
                                    .w(px(col1)).min_w(px(60.0))
                                    .px(px(12.0)).py(px(8.0))
                                    .text_size(px(12.0)).font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.accent).overflow_hidden()
                                    .child(cookie.name)
                            )
                            .child(div().w(px(4.0)))
                            .child(
                                div()
                                    .flex_1().px(px(12.0)).py(px(8.0))
                                    .text_size(px(12.0)).font_family("JetBrains Mono")
                                    .text_color(theme.colors.text_primary).overflow_hidden()
                                    .child(cookie.value)
                            )
                            .child(div().w(px(4.0)))
                            .child(
                                div()
                                    .w(px(col3)).min_w(px(60.0))
                                    .px(px(12.0)).py(px(8.0))
                                    .text_size(px(11.0)).text_color(theme.colors.text_muted).overflow_hidden()
                                    .child(cookie.path.unwrap_or_else(|| "/".to_string()))
                            )
                            .child(div().w(px(4.0)))
                            .child(
                                div()
                                    .w(px(col4)).min_w(px(60.0))
                                    .px(px(12.0)).py(px(8.0))
                                    .text_size(px(10.0))
                                    .text_color(if flags_str.is_empty() { theme.colors.text_muted.opacity(0.5) } else { theme.colors.status_success })
                                    .child(if flags_str.is_empty() { "-".to_string() } else { flags_str })
                            )
                    }))
            )
            .into_any_element()
    }
}
