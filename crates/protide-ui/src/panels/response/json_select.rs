use super::*;

impl ResponsePanel {
    fn estimate_row_height(&self, row: &JsonRow, container_w: f32, row_i: usize) -> f32 {
        let available_w = (container_w - GUTTER_W - row.depth as f32 * INDENT_W - CHEVRON_W)
            .max(JSON_CHAR_W * 10.0);
        let text_w = match &row.kind {
            RowKind::Leaf { key, val, .. } => {
                let kw = key
                    .as_deref()
                    .map(|k| (k.len() + 4) as f32 * JSON_CHAR_W)
                    .unwrap_or(0.0);
                let vw = match val {
                    PrimVal::Str { display, .. } => display.len() as f32 * JSON_CHAR_W,
                    PrimVal::Num(n) => n.len() as f32 * JSON_CHAR_W,
                    _ => JSON_CHAR_W * 5.0,
                };
                kw + vw
            }
            _ => JSON_CHAR_W * 3.0,
        };
        let text_lines = (text_w / available_w).ceil().max(1.0);
        // Expandable strings (> COLLAPSE_CHARS) add a toggle button row.
        // When expanded, they wrap the full text (already counted above).
        match &row.kind {
            RowKind::Leaf {
                val: PrimVal::Str { display, .. },
                ..
            } if display.len() > COLLAPSE_CHARS + 3 => {
                let visible_lines = if self.expanded_strings.contains(&row_i) {
                    text_lines
                } else {
                    1.0 // collapsed: single truncated line
                };
                visible_lines * ROW_H + ROW_H // content + button
            }
            _ => text_lines * ROW_H,
        }
    }

    pub(super) fn json_row_at_y(&self, ey: Pixels) -> Option<usize> {
        let bounds = self.json_tree_bounds?;
        let rel_y = f32::from(ey) - f32::from(bounds.origin.y);
        if rel_y < 0.0 {
            return None;
        }

        // Perf mode: uniform row heights - simple division.
        if self.json_rows.len() > WRAP_MODE_MAX_ROWS {
            let row = (rel_y / ROW_H) as usize;
            return (row < self.json_rows.len()).then_some(row);
        }

        // Wrap mode: rows have variable height. Accumulate estimated heights so a
        // click on the 2nd/3rd visual line of a wrapped row maps to the correct row.
        let container_w = f32::from(bounds.size.width);
        let mut y = 0.0f32;
        for (i, row) in self.json_rows.iter().enumerate() {
            let h = self.estimate_row_height(row, container_w, i);
            if rel_y < y + h {
                return Some(i);
            }
            y += h;
        }
        self.json_rows.len().checked_sub(1)
    }

    pub(super) fn json_row_display_text(&self, row_i: usize) -> &str {
        let Some(row) = self.json_rows.get(row_i) else {
            return "";
        };
        match &row.kind {
            RowKind::Leaf { val, .. } => match val {
                PrimVal::Null => "null",
                PrimVal::Bool(b) => {
                    if *b {
                        "true"
                    } else {
                        "false"
                    }
                }
                PrimVal::Num(n) => n.as_ref(),
                PrimVal::Str { display, .. } => display.as_ref(),
                PrimVal::EmptyArr => "[]",
                PrimVal::EmptyObj => "{}",
            },
            RowKind::Open { arr: true, .. } => "[",
            RowKind::Open { arr: false, .. } => "{",
            RowKind::Close { arr: true } => "]",
            RowKind::Close { arr: false } => "}",
            RowKind::Folded { arr: true, .. } => "[...]",
            RowKind::Folded { arr: false, .. } => "{...}",
        }
    }

    /// Returns a **byte** offset into the row's display text for the value at
    /// pixel-x `ex` (mirrors `hdr_val_byte_at` in `mod.rs`). Must not return a
    /// char-count, since callers (e.g. `copy_json_selection`,
    /// `render_selectable_json_value`) slice the text directly with `&text[..n]`.
    pub(super) fn json_val_char_at_x(&self, ex: Pixels, row_i: usize) -> usize {
        let Some(row) = self.json_rows.get(row_i) else {
            return 0;
        };
        let RowKind::Leaf { .. } = &row.kind else {
            return 0;
        };
        let key_chars = match &row.kind {
            // Rendered as `"key": ` - a *character* count, since it is scaled by
            // JSON_CHAR_W below. `k.len()` (bytes) would over-measure any
            // non-ASCII key and push every click on this row's value leftwards.
            RowKind::Leaf { key: Some(k), .. } => k.chars().count() + 4,
            _ => 0,
        };
        let bounds = self.json_tree_bounds.unwrap_or_default();
        let val_x = f32::from(bounds.origin.x)
            + GUTTER_W
            + (row.depth as f32) * INDENT_W
            + CHEVRON_W
            + (key_chars as f32) * JSON_CHAR_W;
        let char_x = (f32::from(ex) - val_x).max(0.0);
        let char_idx = (char_x / JSON_CHAR_W) as usize;
        let text = self.json_row_display_text(row_i);
        text.char_indices()
            .nth(char_idx)
            .map(|(byte_pos, _)| byte_pos)
            .unwrap_or(text.len())
    }

    pub(super) fn copy_json_selection(&mut self, cx: &mut Context<Self>) {
        let Some(sel) = self.json_sel else { return };
        let n = self.json_rows.len();
        if n == 0 {
            return;
        }
        let (sr, er, so, eo) = if sel.start_row <= sel.end_row {
            (sel.start_row, sel.end_row, sel.start_offset, sel.end_offset)
        } else {
            (sel.end_row, sel.start_row, sel.end_offset, sel.start_offset)
        };
        let mut parts: Vec<String> = Vec::new();
        for i in sr..=er.min(n - 1) {
            let text = self.json_row_display_text(i);
            let tl = text.len();
            // Offsets are clamped to a valid char boundary as a defensive backstop
            // against panicking on `&text[..n]` (see floor_char_boundary docs).
            let chunk = if sr == er {
                let s = floor_char_boundary(text, so.min(eo).min(tl));
                let e = floor_char_boundary(text, so.max(eo).min(tl));
                &text[s..e]
            } else if i == sr {
                &text[floor_char_boundary(text, so.min(tl))..]
            } else if i == er {
                &text[..floor_char_boundary(text, eo.min(tl))]
            } else {
                text
            };
            if !chunk.is_empty() {
                parts.push(chunk.to_string());
            }
        }
        let combined = parts.join("\n");
        if !combined.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(combined));
            self.show_copy_feedback(CopyFeedback::Body, cx);
        }
    }
}
