use super::*;

impl ResponsePanel {
    pub(super) fn render_headers_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let header_count = response.headers.len();

        if header_count == 0 {
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
                                .child(icon(ICON_COPY, ICON_MD, theme.colors.text_muted.opacity(0.5)))
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("No headers in response")
                        )
                )
                .into_any_element();
        }

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Header toolbar
            .child({
                let header_is_copied = matches!(self.copy_feedback, Some(CopyFeedback::Headers) | Some(CopyFeedback::HdrVal));
                let headers_text: String = response.headers
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n");
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .relative()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .bg(theme.colors.accent.opacity(0.12))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.accent)
                                    .child(format!("{}", header_count))
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("response headers")
                            )
                    )
                    // Copy headers button — deferred so it paints above the headers table below
                    .child(deferred(
                        div()
                            .id("copy-headers-btn")
                            .absolute()
                            .right_0()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(10.0))
                            .py(px(5.0))
                            .text_size(px(11.0))
                            .when(header_is_copied, |el| el.text_color(theme.colors.status_success).border_color(theme.colors.status_success))
                            .when(!header_is_copied, |el| el.text_color(theme.colors.text_secondary).border_color(theme.colors.border))
                            .cursor_pointer()
                            .border_1()
                            .bg(theme.colors.bg_primary)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(headers_text.clone()));
                                this.show_copy_feedback(CopyFeedback::Headers, cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .when(header_is_copied, |el| el.child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success)))
                                    .when(!header_is_copied, |el| el.child(icon(ICON_COPY, ICON_MD, theme.colors.text_secondary)))
                            )
                            .child(if header_is_copied { "Copied!" } else { "Copy" })
                    ).with_priority(1))
            })
            // Headers table
            .child({
                let weak = cx.weak_entity();
                div()
                    .id("hdr-table")
                    .w_full()
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    // Capture bounds for mouse→row/char hit-testing
                    .child(
                        canvas(
                            move |bounds, _, cx| {
                                weak.update(cx, |this, _| { this.hdr_table_bounds = Some(bounds); }).ok();
                            },
                            |_, _, _, _| {},
                        )
                        .absolute().top_0().left_0().size_full()
                    )
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _, cx| {
                        let Some(row) = this.hdr_row_at(event.position.y) else { return };
                        let bounds = this.hdr_table_bounds.unwrap_or_default();
                        let val_col_x = f32::from(bounds.origin.x) + this.resp_header_col1_w + HDR_SPACER_W;
                        if f32::from(event.position.x) < val_col_x { return; }
                        let byte = this.hdr_val_byte_at(event.position.x, row);
                        this.hdr_sel = Some(HdrSel { row, range: (byte, byte), selecting: true });
                        cx.notify();
                    }))
                    .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                        let Some((row, old_range)) = this.hdr_sel
                            .as_ref()
                            .filter(|s| s.selecting)
                            .map(|s| (s.row, s.range))
                        else { return };
                        let new_end = this.hdr_val_byte_at(event.position.x, row);
                        if selection_changed(Some(old_range), old_range.0, new_end) {
                            if let Some(sel) = this.hdr_sel.as_mut() {
                                sel.range.1 = new_end;
                            }
                            cx.notify();
                        }
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        if let Some(sel) = this.hdr_sel {
                            if sel.selecting {
                                let (a, b) = sel.range;
                                let (s, e) = (a.min(b), a.max(b));
                                if s != e {
                                    if let Some((_, val)) = this.response.as_ref()
                                        .and_then(|r| r.headers.get(sel.row))
                                    {
                                        let text = val[s.min(val.len())..e.min(val.len())].to_string();
                                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                                        this.show_copy_feedback(CopyFeedback::HdrVal, cx);
                                    }
                                }
                                this.hdr_sel = Some(HdrSel { selecting: false, ..sel });
                            }
                        }
                        cx.notify();
                    }))
                    // Table header
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .bg(theme.colors.bg_secondary)
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .child(
                                div()
                                    .w(px(self.resp_header_col1_w))
                                    .min_w(px(60.0))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("NAME")
                            )
                            .child(self.render_col_drag_handle(0, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("VALUE")
                            )
                    )
                    // Table rows
                    .children(response.headers.iter().enumerate().map(|(i, (key, value))| {
                        let is_last = i == header_count - 1;
                        let col1_w = self.resp_header_col1_w;
                        let sel_range = if self.hdr_sel.as_ref().map(|s| s.row == i).unwrap_or(false) {
                            self.hdr_sel.map(|s| {
                                let (a, b) = s.range;
                                if a <= b { (a, b) } else { (b, a) }
                            })
                        } else {
                            None
                        };
                        div()
                            .w_full()
                            .flex()
                            .when(i % 2 == 0, |el| el.bg(theme.colors.bg_tertiary.opacity(0.3)))
                            .when(!is_last, |el| el.border_b_1().border_color(theme.colors.border.opacity(0.5)))
                            .child(
                                div()
                                    .w(px(col1_w))
                                    .min_w(px(60.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.accent)
                                    .overflow_hidden()
                                    .child(key.clone())
                            )
                            .child(div().w(px(4.0)))
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .overflow_hidden()
                                    .child(selectable_text_element(
                                        gpui::ElementId::Integer(i as u64),
                                        SharedString::from(value.as_str()),
                                        sel_range,
                                        theme.colors.text_primary,
                                        theme.colors.accent.opacity(0.3),
                                        12.0,
                                    ))
                            )
                    }))
            })
            .into_any_element()
    }
}
