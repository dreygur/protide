use super::*;

pub(super) fn render_json_row(
    line_num:  usize,
    row:       &JsonRow,
    colors:    &crate::theme::Colors,
    weak:      WeakEntity<ResponsePanel>,
    wrap_mode: bool,
    expanded:  bool,
    sel_range: Option<&SelectionRange>,
) -> gpui::AnyElement {
    let guide_color = colors.border;
    let hover_bg    = colors.hover_overlay;
    let depth       = row.depth;

    // Gutter: right-aligned line number
    // In wrap mode: top-padded so the number aligns with the first text line.
    let gutter = if wrap_mode {
        div()
            .w(px(GUTTER_W)).flex_shrink_0()
            .flex().items_start().justify_end()
            .pt(px(4.0)).pr(px(8.0))
            .text_size(px(GUTTER_FONT)).text_color(colors.text_muted)
            .child(SharedString::from(line_num.to_string()))
    } else {
        div()
            .w(px(GUTTER_W)).h(px(ROW_H)).flex_shrink_0()
            .flex().items_center().justify_end().pr(px(8.0))
            .text_size(px(GUTTER_FONT)).text_color(colors.text_muted)
            .child(SharedString::from(line_num.to_string()))
    };

    // Indent-guide pillars: one per depth level.
    // In wrap mode each pillar uses self_stretch() so it fills the full (variable) row height,
    // and the 1px inner line uses h_full() to span that stretched height.
    let guides: Vec<gpui::AnyElement> = (0..depth).map(|_| {
        let g = div().w(px(INDENT_W)).flex_shrink_0().flex().justify_center();
        let g = if wrap_mode { g.self_stretch() } else { g.h(px(ROW_H)) };
        g.child(
            div().w(px(1.0)).h_full().flex_shrink_0().bg(guide_color),
        )
        .into_any_element()
    }).collect();

    // Chevron column — top-aligned in wrap mode so it sits next to the first text line
    let is_open   = matches!(&row.kind, RowKind::Open { .. });
    let is_folded = matches!(&row.kind, RowKind::Folded { .. });
    let chevron = {
        let c = div().w(px(CHEVRON_W)).flex_shrink_0().flex().justify_center();
        let c = if wrap_mode {
            c.items_start().pt(px(2.0))
        } else {
            c.items_center().h(px(ROW_H))
        };
        c.when(is_open,   |el| el.child(icon(ICON_CHEVRON_DOWN,  ICON_SM, colors.text_secondary)))
         .when(is_folded, |el| el.child(icon(ICON_CHEVRON_RIGHT, ICON_SM, colors.text_secondary)))
    };

    let key_color   = colors.info;
    let colon_color = colors.text_secondary.opacity(0.45);
    let key_span = move |k: &str| -> gpui::AnyElement {
        div()
            .flex_shrink_0().flex().items_center().whitespace_nowrap()
            .child(div().text_color(key_color).child(SharedString::from(format!("\"{}\"", k))))
            .child(div().flex_shrink_0().text_color(colon_color).child(": "))
            .into_any_element()
    };

    let c_secondary = colors.text_secondary;
    let c_patch     = colors.method_patch;
    let c_put       = colors.method_put;
    let c_success   = colors.status_success;
    let c_accent    = colors.accent;

    let content: gpui::AnyElement = match &row.kind {
        RowKind::Leaf { key, val, .. } => {
            // Display text is pre-computed at flatten time — no allocs on the render path.
            let (txt, col, is_str): (SharedString, _, bool) = match val {
                PrimVal::Null     => (SharedString::new_static("null"),                  c_secondary, false),
                PrimVal::Bool(b)  => (SharedString::new_static(if *b { "true" } else { "false" }), c_patch, false),
                PrimVal::Num(n)   => (n.clone(),                                         c_put,       false),
                PrimVal::Str { display, .. } => (display.clone(),                        c_success,   true),
                PrimVal::EmptyArr => (SharedString::new_static("[]"),                    c_secondary, false),
                PrimVal::EmptyObj => (SharedString::new_static("{}"),                    c_secondary, false),
            };

            // Long strings in wrap mode: show truncated text + expand/collapse toggle
            if wrap_mode && is_str && txt.len() > COLLAPSE_CHARS + 3 {
                let row_idx  = line_num - 1;
                let weak_str = weak.clone();
                let display: SharedString = if expanded {
                    txt.clone()
                } else {
                    // Safe byte boundary: stay within ASCII quotes wrapping
                    let cut = txt.char_indices()
                        .nth(COLLAPSE_CHARS + 1)
                        .map(|(i, _)| i)
                        .unwrap_or(txt.len());
                    SharedString::new(format!("{}…\"", &txt[..cut]))
                };
                let btn_label: &'static str = if expanded { "▲ collapse" } else { "▼ show more" };

                let value_block = div()
                    .flex_1().min_w(px(0.))
                    .flex().flex_col().gap(px(2.0))
                    .child(div().whitespace_normal().text_color(col).child(display))
                    .child(
                        div()
                            .id(SharedString::from(format!("str-toggle-{}", row_idx)))
                            .cursor_pointer()
                            .text_size(px(9.0))
                            .text_color(c_secondary.opacity(0.6))
                            .hover(|s| s.text_color(c_accent))
                            .on_click(move |_, _, cx| {
                                weak_str.update(cx, |this, cx| {
                                    if expanded {
                                        this.expanded_strings.remove(&row_idx);
                                    } else {
                                        this.expanded_strings.insert(row_idx);
                                    }
                                    cx.notify();
                                }).ok();
                            })
                            .child(btn_label)
                    );

                div()
                    .flex_1().min_w(px(0.)).flex().items_start()
                    .when_some(key.as_deref(), |el, k| el.child(key_span(k)))
                    .child(value_block)
                    .into_any_element()

            } else if wrap_mode {
                div()
                    .flex_1().min_w(px(0.)).flex().items_start()
                    .whitespace_normal()
                    .when_some(key.as_deref(), |el, k| el.child(key_span(k)))
                    .child(
                        div().flex_1().min_w(px(0.))
                            .child(render_selectable_json_value(
                                gpui::ElementId::Integer(line_num as u64),
                                &txt,
                                sel_range,
                                line_num - 1,
                                col,
                                c_accent.opacity(0.3),
                                ROW_FONT,
                            ))
                    )
                    .into_any_element()

            } else {
                // Perf / uniform_list mode: single line, truncate with ellipsis
                div()
                    .flex_1().min_w(px(0.)).overflow_hidden().flex().items_center().whitespace_nowrap()
                    .when_some(key.as_deref(), |el, k| el.child(key_span(k)))
                    .child(
                        div().flex_1().min_w(px(0.)).overflow_hidden().text_ellipsis().whitespace_nowrap()
                            .text_color(col)
                            .child(txt),  // SharedString — no alloc
                    )
                    .into_any_element()
            }
        }

        RowKind::Open { key, arr, .. } => {
            let bracket = if *arr { "[" } else { "{" };
            div()
                .flex_1().min_w(px(0.)).overflow_hidden().flex().items_center().whitespace_nowrap()
                .when_some(key.as_deref(), |el, k| el.child(key_span(k)))
                .child(div().flex_shrink_0().text_color(c_secondary).child(bracket))
                .into_any_element()
        }
        RowKind::Close { arr, .. } => {
            let bracket = if *arr { "]" } else { "}" };
            div()
                .flex_1().overflow_hidden().flex().items_center().whitespace_nowrap()
                .child(div().flex_shrink_0().text_color(c_secondary).child(bracket))
                .into_any_element()
        }
        RowKind::Folded { key, arr, count, .. } => {
            let (open, close) = if *arr { ("[", "]") } else { ("{", "}") };
            let summary = SharedString::from(format!(
                " {} {} ", count, if *arr { "items" } else { "keys" }
            ));
            let dim = c_secondary.opacity(0.5);
            div()
                .flex_1().overflow_hidden().flex().items_center().whitespace_nowrap()
                .when_some(key.as_deref(), |el, k| el.child(key_span(k)))
                .child(div().flex_shrink_0().text_color(c_secondary).child(open))
                .child(div().flex_shrink_0().text_color(dim).child(summary))
                .child(div().flex_shrink_0().text_color(c_secondary).child(close))
                .into_any_element()
        }
    };

    // Click path for collapsible rows
    let click_path: Option<String> = match &row.kind {
        RowKind::Open   { path, .. } => Some(path.clone()),
        RowKind::Folded { path, .. } => Some(path.clone()),
        _ => None,
    };
    let can_click = click_path.is_some();

    // Build right-click context menu items
    let ctx_items: Vec<(String, String)> = match &row.kind {
        RowKind::Leaf { key, val, path } => {
            let val_str = match val {
                PrimVal::Null     => "null".to_string(),
                PrimVal::Bool(b)  => if *b { "true" } else { "false" }.to_string(),
                PrimVal::Num(n)   => n.to_string(),
                PrimVal::Str { raw, .. } => raw.clone(),  // raw = unquoted original value
                PrimVal::EmptyArr => "[]".to_string(),
                PrimVal::EmptyObj => "{}".to_string(),
            };
            let mut items = vec![("Copy Value".to_string(), val_str)];
            if let Some(k) = key { items.push(("Copy Key".to_string(), k.clone())); }
            items.push(("Copy Path".to_string(), path.clone()));
            items
        }
        RowKind::Open { key, path, .. } | RowKind::Folded { key, path, .. } => {
            let mut items = vec![];
            if let Some(k) = key { items.push(("Copy Key".to_string(), k.clone())); }
            items.push(("Copy Path".to_string(), path.clone()));
            items
        }
        RowKind::Close { .. } => vec![],
    };

    // Outer row: variable height in wrap mode, fixed in perf mode.
    // items_start keeps gutter/chevron top-aligned; guide pillars override via self_stretch().
    let row_div = if wrap_mode {
        div()
            .id(line_num).w_full().min_h(px(ROW_H))
            .flex().items_start()
            .when(can_click, |el| el.cursor_pointer())
            .hover(|s| s.bg(hover_bg))
    } else {
        div()
            .id(line_num).w_full().h(px(ROW_H))
            .flex().items_center().overflow_hidden()
            .when(can_click, |el| el.cursor_pointer())
            .hover(|s| s.bg(hover_bg))
    };

    let row_div = row_div.child(gutter).children(guides).child(chevron).child(content);

    // Attach right-click handler if we have context items
    let row_div = if !ctx_items.is_empty() {
        let items = ctx_items;
        let weak_cm = weak.clone();
        row_div.on_mouse_down(MouseButton::Right, move |event: &gpui::MouseDownEvent, _, cx| {
            let pos = event.position;
            let items_c = items.clone();
            weak_cm.update(cx, |this, cx| {
                let lx = f32::from(pos.x) - f32::from(this.bounds_origin.x);
                let ly = f32::from(pos.y) - f32::from(this.bounds_origin.y);
                this.json_context_menu = Some(JsonCtxMenu { x: lx, y: ly, items: items_c });
                cx.notify();
            }).ok();
        })
    } else {
        row_div
    };

    if let Some(path) = click_path {
        let weak_c = weak;
        row_div
            .on_click(move |_, _, cx| {
                weak_c.update(cx, |this, cx| {
                    this.toggle_json_collapse(path.clone(), cx);
                }).ok();
            })
            .into_any_element()
    } else {
        row_div.into_any_element()
    }
}
