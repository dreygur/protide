use super::*;
use super::render_json_row::render_json_row;

impl ResponsePanel {
    pub(super) fn render_json_tree(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let row_count = self.json_rows.len();
        if row_count == 0 {
            return div().into_any_element();
        }

        let use_wrap = row_count <= WRAP_MODE_MAX_ROWS;

        div()
            .flex_1()
            .w_full()
            .overflow_hidden()
            .child(if use_wrap {
                // ── Wrap mode: plain scrollable div, variable row heights ──────────
                let colors   = theme::current(cx).colors.clone();
                let weak     = cx.weak_entity();
                let expanded = &self.expanded_strings;
                let sel      = self.json_sel.as_ref();
                let rows: Vec<gpui::AnyElement> = self.json_rows.iter().enumerate()
                    .map(|(i, row)| render_json_row(
                        i + 1, row, &colors, weak.clone(),
                        true, expanded.contains(&i), sel,
                    ))
                    .collect();
                let weak_bounds = cx.weak_entity();
                div()
                    .id("json-tree")
                    .size_full()
                    .overflow_scroll()
                    .font_family(SharedString::from("JetBrains Mono"))
                    .text_size(px(ROW_FONT))
                    .track_scroll(&self.json_scroll_handle_div)
                    // Capture bounds each frame for mouse→row hit-testing
                    .child(
                        canvas(
                            move |bounds, _, cx| {
                                weak_bounds.update(cx, |this, _| { this.json_tree_bounds = Some(bounds); }).ok();
                            },
                            |_, _, _, _| {},
                        )
                        .absolute().top_0().left_0().size_full()
                    )
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _, cx| {
                        let Some(row_i) = this.json_row_at_y(event.position.y) else { return };
                        let col = this.json_val_char_at_x(event.position.x, row_i);
                        this.json_sel = Some(SelectionRange::new(row_i, col, row_i, col));
                        this.json_selecting = true;
                        cx.notify();
                    }))
                    .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                        if !this.json_selecting { return; }
                        let Some(sel) = this.json_sel else { return };
                        let new_row = this.json_row_at_y(event.position.y)
                            .unwrap_or_else(|| this.json_rows.len().saturating_sub(1));
                        let new_col = this.json_val_char_at_x(event.position.x, new_row);
                        if selection_changed(
                            Some((sel.end_offset, sel.end_row)),
                            new_col, new_row,
                        ) {
                            this.json_sel = Some(SelectionRange::new(sel.start_row, sel.start_offset, new_row, new_col));
                            cx.notify();
                        }
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        if this.json_selecting {
                            this.json_selecting = false;
                            this.copy_json_selection(cx);
                        }
                    }))
                    .children(rows)
                    .into_any_element()
            } else {
                // ── Perf mode: uniform_list (fixed ROW_H, no wrapping) ────────────
                // Selection not supported in perf mode (rows are virtualized).
                uniform_list(
                    "json-tree",
                    row_count,
                    cx.processor(|this, range: std::ops::Range<usize>, _window, cx| {
                        let colors = theme::current(cx).colors.clone();
                        let weak   = cx.weak_entity();
                        range.map(|i| {
                            let row = this.json_rows[i].clone();
                            render_json_row(i + 1, &row, &colors, weak.clone(), false, false, None)
                        })
                        .collect::<Vec<_>>()
                    }),
                )
                .font_family(SharedString::from("JetBrains Mono"))
                .text_size(px(ROW_FONT))
                .size_full()
                .track_scroll(&self.json_scroll_handle)
                .into_any_element()
            })
            .into_any_element()
    }
}
