use super::*;

// ─── JSON tree types ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum PrimVal {
    Null,
    Bool(bool),
    /// Pre-formatted display string, e.g. "42.5". Raw value == display for numbers.
    Num(SharedString),
    /// `raw` is the original JSON string value (used for clipboard copy).
    /// `display` is pre-sanitized and quoted: `"\"hello world\""`.
    Str { raw: String, display: SharedString },
    EmptyArr,
    EmptyObj,
}

#[derive(Clone, Debug)]
pub enum RowKind {
    Leaf   { key: Option<String>, val: PrimVal, path: String },
    Open   { key: Option<String>, arr: bool, path: String },
    Close  { arr: bool },
    Folded { key: Option<String>, arr: bool, count: usize, path: String },
}

/// Items shown in the JSON right-click context menu
#[derive(Clone, Debug)]
pub struct JsonCtxMenu {
    pub x: f32,
    pub y: f32,
    pub items: Vec<(String, String)>, // (label, clipboard_value)
}

#[derive(Clone, Debug)]
pub struct JsonRow {
    pub depth: usize,
    pub kind:  RowKind,
}

pub fn flatten_json(
    value:     &serde_json::Value,
    depth:     usize,
    key:       Option<String>,
    path:      &str,
    collapsed: &std::collections::HashSet<String>,
    rows:      &mut Vec<JsonRow>,
) {
    use serde_json::Value;
    let leaf_path = path.to_string();
    match value {
        Value::Null    => rows.push(JsonRow { depth, kind: RowKind::Leaf { key, val: PrimVal::Null,         path: leaf_path } }),
        Value::Bool(b) => rows.push(JsonRow { depth, kind: RowKind::Leaf { key, val: PrimVal::Bool(*b),    path: leaf_path } }),
        Value::Number(n) => rows.push(JsonRow { depth, kind: RowKind::Leaf { key, val: PrimVal::Num(SharedString::new(n.to_string())), path: leaf_path } }),
        Value::String(s) => {
            let san: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
            let display = SharedString::new(format!("\"{}\"", san));
            rows.push(JsonRow { depth, kind: RowKind::Leaf { key, val: PrimVal::Str { raw: s.clone(), display }, path: leaf_path } });
        }
        Value::Array(arr) if arr.is_empty() => {
            rows.push(JsonRow { depth, kind: RowKind::Leaf { key, val: PrimVal::EmptyArr, path: leaf_path } });
        }
        Value::Array(arr) => {
            let count = arr.len();
            let p = path.to_string();
            if collapsed.contains(path) {
                rows.push(JsonRow { depth, kind: RowKind::Folded { key, arr: true, count, path: p } });
                return;
            }
            rows.push(JsonRow { depth, kind: RowKind::Open { key, arr: true, path: p } });
            for (i, item) in arr.iter().enumerate() {
                flatten_json(item, depth + 1, None, &format!("{}/{}", path, i), collapsed, rows);
            }
            rows.push(JsonRow { depth, kind: RowKind::Close { arr: true } });
        }
        Value::Object(obj) if obj.is_empty() => {
            rows.push(JsonRow { depth, kind: RowKind::Leaf { key, val: PrimVal::EmptyObj, path: leaf_path } });
        }
        Value::Object(obj) => {
            let count = obj.len();
            let p = path.to_string();
            if collapsed.contains(path) {
                rows.push(JsonRow { depth, kind: RowKind::Folded { key, arr: false, count, path: p } });
                return;
            }
            rows.push(JsonRow { depth, kind: RowKind::Open { key, arr: false, path: p } });
            for (k, v) in obj {
                flatten_json(v, depth + 1, Some(k.clone()), &format!("{}/{}", path, k), collapsed, rows);
            }
            rows.push(JsonRow { depth, kind: RowKind::Close { arr: false } });
        }
    }
}

impl ResponsePanel {
    pub(super) fn toggle_json_collapse(&mut self, path: String, cx: &mut Context<Self>) {
        if self.json_tree_collapsed.contains(&path) {
            self.json_tree_collapsed.remove(&path);
        } else {
            self.json_tree_collapsed.insert(path);
        }
        self.json_sel = None;
        self.rebuild_json_rows();
        cx.notify();
    }

    pub(super) fn rebuild_json_rows(&mut self) {
        self.json_rows.clear();
        if let Some(json) = &self.json_value {
            flatten_json(json, 0, None, "", &self.json_tree_collapsed, &mut self.json_rows);
        }
    }

    pub(super) fn json_row_at_y(&self, ey: Pixels) -> Option<usize> {
        let bounds = self.json_tree_bounds?;
        let rel_y = f32::from(ey) - f32::from(bounds.origin.y);
        if rel_y < 0.0 { return None; }

        // Perf mode: uniform row heights — simple division.
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

    fn estimate_row_height(&self, row: &JsonRow, container_w: f32, row_i: usize) -> f32 {
        let available_w = (container_w - GUTTER_W - row.depth as f32 * INDENT_W - CHEVRON_W)
            .max(JSON_CHAR_W * 10.0);
        let text_w = match &row.kind {
            RowKind::Leaf { key, val, .. } => {
                let kw = key.as_deref().map(|k| (k.len() + 4) as f32 * JSON_CHAR_W).unwrap_or(0.0);
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
        let btn_extra = match &row.kind {
            RowKind::Leaf { val: PrimVal::Str { display, .. }, .. }
                if display.len() > COLLAPSE_CHARS + 3 =>
            {
                let visible_lines = if self.expanded_strings.contains(&row_i) {
                    text_lines
                } else {
                    1.0  // collapsed: single truncated line
                };
                visible_lines * ROW_H + ROW_H  // content + button
            }
            _ => text_lines * ROW_H,
        };
        btn_extra
    }

    pub(super) fn json_val_char_at_x(&self, ex: Pixels, row_i: usize) -> usize {
        let Some(row) = self.json_rows.get(row_i) else { return 0 };
        let key_chars = match &row.kind {
            RowKind::Leaf { key: Some(k), .. } => k.len() + 4, // "key":
            _ => 0,
        };
        let bounds = self.json_tree_bounds.unwrap_or_default();
        let val_x = f32::from(bounds.origin.x) + GUTTER_W + (row.depth as f32) * INDENT_W + CHEVRON_W + (key_chars as f32) * JSON_CHAR_W;
        let char_x = (f32::from(ex) - val_x).max(0.0);
        let max_len = match &row.kind {
            RowKind::Leaf { val, .. } => match val {
                PrimVal::Null => 4,
                PrimVal::Bool(b) => if *b { 4 } else { 5 },
                PrimVal::Num(n) => n.len(),
                PrimVal::Str { display, .. } => display.len(),
                PrimVal::EmptyArr | PrimVal::EmptyObj => 2,
            },
            _ => 0,
        };
        ((char_x / JSON_CHAR_W) as usize).min(max_len)
    }

    pub(super) fn json_row_display_text(&self, row_i: usize) -> &str {
        let Some(row) = self.json_rows.get(row_i) else { return "" };
        match &row.kind {
            RowKind::Leaf { val, .. } => match val {
                PrimVal::Null => "null",
                PrimVal::Bool(b) => if *b { "true" } else { "false" },
                PrimVal::Num(n) => n.as_ref(),
                PrimVal::Str { display, .. } => display.as_ref(),
                PrimVal::EmptyArr => "[]",
                PrimVal::EmptyObj => "{}",
            },
            RowKind::Open { arr: true, .. }   => "[",
            RowKind::Open { arr: false, .. }  => "{",
            RowKind::Close { arr: true }       => "]",
            RowKind::Close { arr: false }      => "}",
            RowKind::Folded { arr: true, .. }  => "[...]",
            RowKind::Folded { arr: false, .. } => "{...}",
        }
    }

    pub(super) fn copy_json_selection(&mut self, cx: &mut Context<Self>) {
        let Some(sel) = self.json_sel else { return };
        let n = self.json_rows.len();
        if n == 0 { return; }
        let (sr, er, so, eo) = if sel.start_row <= sel.end_row {
            (sel.start_row, sel.end_row, sel.start_offset, sel.end_offset)
        } else {
            (sel.end_row, sel.start_row, sel.end_offset, sel.start_offset)
        };
        let mut parts: Vec<String> = Vec::new();
        for i in sr..=er.min(n - 1) {
            let text = self.json_row_display_text(i);
            let tl = text.len();
            let chunk = if sr == er {
                &text[so.min(tl)..eo.min(tl)]
            } else if i == sr {
                &text[so.min(tl)..]
            } else if i == er {
                &text[..eo.min(tl)]
            } else {
                text
            };
            if !chunk.is_empty() { parts.push(chunk.to_string()); }
        }
        let combined = parts.join("\n");
        if !combined.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(combined));
            self.show_copy_feedback(CopyFeedback::Body, cx);
        }
    }
}
