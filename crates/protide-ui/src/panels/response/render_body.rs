use super::*;

impl ResponsePanel {
    pub(super) fn render_body_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

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

        let view_mode = self.body_view_mode;
        let fb = self.formatted_body.clone();
        let fl = self.formatted_lang.clone();
        let rb = self.raw_body.clone();

        let line_count = self.body_viewer.read(cx).value().lines().count();
        let is_copied = self.copy_feedback == Some(CopyFeedback::Body);

        let tab_btn = {
            let accent = theme.colors.accent;
            let border = theme.colors.border;
            let bg_secondary = theme.colors.bg_secondary;
            let bg_tertiary = theme.colors.bg_tertiary;
            let text_secondary = theme.colors.text_secondary;
            move |label: &'static str, is_active: bool| {
                div()
                    .px(px(10.0))
                    .py(px(4.0))
                    .border_1()
                    .border_color(if is_active { accent } else { border })
                    .bg(if is_active { accent.opacity(0.10) } else { bg_secondary })
                    .text_size(px(11.0))
                    .font_weight(if is_active { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                    .text_color(if is_active { accent } else { text_secondary })
                    .cursor_pointer()
                    .hover(move |s| s.bg(bg_tertiary))
                    .child(label)
            }
        };

        let fb1 = fb.clone(); let fl1 = fl.clone();
        let rb1 = rb.clone();
        let fb2 = fb.clone(); let fl2 = fl.clone();

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            // ── Row 1: view-mode tabs (left) + copy button (right) ──
            .child(
                div()
                    .flex_shrink_0()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pb(px(6.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                tab_btn("Pretty", view_mode == BodyViewMode::Pretty)
                                    .id("bvm-pretty")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.body_view_mode = BodyViewMode::Pretty;
                                        this.body_pending = Some((fb1.clone(), fl1.clone()));
                                        cx.notify();
                                    }))
                            )
                            .child(
                                tab_btn("Raw", view_mode == BodyViewMode::Raw)
                                    .id("bvm-raw")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.body_view_mode = BodyViewMode::Raw;
                                        this.body_pending = Some((rb1.clone(), String::new()));
                                        cx.notify();
                                    }))
                            )
                            .child(
                                tab_btn("Preview", view_mode == BodyViewMode::Preview)
                                    .id("bvm-preview")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.body_view_mode = BodyViewMode::Preview;
                                        this.body_pending = Some((fb2.clone(), fl2.clone()));
                                        cx.notify();
                                    }))
                            )
                    )
                    .child(
                        div()
                            .id("copy-body-btn")
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(10.0))
                            .py(px(4.0))
                            .text_size(px(11.0))
                            .border_1()
                            .when(is_copied, |el| el
                                .text_color(theme.colors.status_success)
                                .border_color(theme.colors.status_success))
                            .when(!is_copied, |el| el
                                .text_color(theme.colors.text_secondary)
                                .border_color(theme.colors.border))
                            .cursor_pointer()
                            .bg(theme.colors.bg_primary)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            .on_click(cx.listener(|this, _, _, cx| {
                                let content = this.body_viewer.read(cx).value().to_string();
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
                    )
            )
            // ── Row 2: metrics ──────────────────────────────────────────
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .pb(px(6.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child(format!("{} lines", line_count))
                    )
                    .when(view_mode == BodyViewMode::Pretty, |el| {
                        el.child(
                            div()
                                .text_size(px(10.0))
                                .text_color(theme.colors.text_muted.opacity(0.6))
                                .child("⌘F / Ctrl+F to search")
                        )
                    })
            )
            // ── Row 3: body content (flex_1, scrolls internally) ───────
            .child(match view_mode {
                BodyViewMode::Pretty => {
                    if self.json_value.is_some() {
                        self.render_json_tree(cx)
                    } else {
                        div()
                            .flex_1()
                            .w_full()
                            .overflow_hidden()
                            .child(Input::new(&self.body_viewer).disabled(true).appearance(false).h_full())
                            .into_any_element()
                    }
                }
                BodyViewMode::Raw => {
                    div()
                        .flex_1()
                        .w_full()
                        .overflow_hidden()
                        .child(Input::new(&self.body_viewer).disabled(true).appearance(false).h_full())
                        .into_any_element()
                }
                BodyViewMode::Preview => {
                    self.render_html_preview(&response.body, cx)
                }
            })
            .into_any_element()
    }
}
