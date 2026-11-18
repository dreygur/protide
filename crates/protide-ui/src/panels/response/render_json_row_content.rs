use super::*;

pub(super) fn render_json_row_content(
    line_num: usize,
    row: &JsonRow,
    colors: &crate::theme::Colors,
    weak: &WeakEntity<ResponsePanel>,
    wrap_mode: bool,
    expanded: bool,
    sel_range: Option<&SelectionRange>,
) -> gpui::AnyElement {
    let key_color = colors.info;
    let colon_color = colors.text_secondary.opacity(0.45);
    let key_span = move |k: &str| -> gpui::AnyElement {
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .whitespace_nowrap()
            .child(
                div()
                    .text_color(key_color)
                    .child(SharedString::from(format!("\"{}\"", k))),
            )
            .child(div().flex_shrink_0().text_color(colon_color).child(": "))
            .into_any_element()
    };

    let c_secondary = colors.text_secondary;
    let c_patch = colors.method_patch;
    let c_put = colors.method_put;
    let c_success = colors.status_success;
    let c_accent = colors.accent;

    match &row.kind {
        RowKind::Leaf { key, val, .. } => {
            // Display text is pre-computed at flatten time - no allocs on the render path.
            let (txt, col, is_str): (SharedString, _, bool) = match val {
                PrimVal::Null => (SharedString::new_static("null"), c_secondary, false),
                PrimVal::Bool(b) => (
                    SharedString::new_static(if *b { "true" } else { "false" }),
                    c_patch,
                    false,
                ),
                PrimVal::Num(n) => (n.clone(), c_put, false),
                PrimVal::Str { display, .. } => (display.clone(), c_success, true),
                PrimVal::EmptyArr => (SharedString::new_static("[]"), c_secondary, false),
                PrimVal::EmptyObj => (SharedString::new_static("{}"), c_secondary, false),
            };

            // Long strings in wrap mode: show truncated text + expand/collapse toggle
            if wrap_mode && is_str && txt.len() > COLLAPSE_CHARS + 3 {
                let row_idx = line_num - 1;
                let weak_str = weak.clone();
                let display: SharedString = if expanded {
                    txt.clone()
                } else {
                    // Safe byte boundary: stay within ASCII quotes wrapping
                    let cut = txt
                        .char_indices()
                        .nth(COLLAPSE_CHARS + 1)
                        .map(|(i, _)| i)
                        .unwrap_or(txt.len());
                    SharedString::new(format!("{}…\"", &txt[..cut]))
                };
                let btn_label: &'static str = if expanded {
                    "▲ collapse"
                } else {
                    "▼ show more"
                };

                let value_block = div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(div().whitespace_normal().text_color(col).child(display))
                    .child(
                        div()
                            .id(SharedString::from(format!("str-toggle-{}", row_idx)))
                            .cursor_pointer()
                            .text_size(px(9.0))
                            .text_color(c_secondary.opacity(0.6))
                            .hover(|s| s.text_color(c_accent))
                            .on_click(move |_, _, cx| {
                                weak_str
                                    .update(cx, |this, cx| {
                                        if expanded {
                                            this.expanded_strings.remove(&row_idx);
                                        } else {
                                            this.expanded_strings.insert(row_idx);
                                        }
                                        cx.notify();
                                    })
                                    .ok();
                            })
                            .child(btn_label),
                    );

                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .items_start()
                    .when_some(key.as_deref(), |el, k| el.child(key_span(k)))
                    .child(value_block)
                    .into_any_element()
            } else if wrap_mode {
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .items_start()
                    .whitespace_normal()
                    .when_some(key.as_deref(), |el, k| el.child(key_span(k)))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .child(render_selectable_json_value(
                                gpui::ElementId::Integer(line_num as u64),
                                &txt,
                                sel_range,
                                line_num - 1,
                                col,
                                c_accent.opacity(0.3),
                                ROW_FONT,
                            )),
                    )
                    .into_any_element()
            } else {
                // Perf / uniform_list mode: single line, truncate with ellipsis
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .whitespace_nowrap()
                    .when_some(key.as_deref(), |el, k| el.child(key_span(k)))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_color(col)
                            .child(txt), // SharedString - no alloc
                    )
                    .into_any_element()
            }
        }

        RowKind::Open { key, arr, .. } => {
            let bracket = if *arr { "[" } else { "{" };
            div()
                .flex_1()
                .min_w(px(0.))
                .overflow_hidden()
                .flex()
                .items_center()
                .whitespace_nowrap()
                .when_some(key.as_deref(), |el, k| el.child(key_span(k)))
                .child(div().flex_shrink_0().text_color(c_secondary).child(bracket))
                .into_any_element()
        }
        RowKind::Close { arr, .. } => {
            let bracket = if *arr { "]" } else { "}" };
            div()
                .flex_1()
                .overflow_hidden()
                .flex()
                .items_center()
                .whitespace_nowrap()
                .child(div().flex_shrink_0().text_color(c_secondary).child(bracket))
                .into_any_element()
        }
        RowKind::Folded {
            key, arr, count, ..
        } => {
            let (open, close) = if *arr { ("[", "]") } else { ("{", "}") };
            let summary = SharedString::from(format!(
                " {} {} ",
                count,
                if *arr { "items" } else { "keys" }
            ));
            let dim = c_secondary.opacity(0.5);
            div()
                .flex_1()
                .overflow_hidden()
                .flex()
                .items_center()
                .whitespace_nowrap()
                .when_some(key.as_deref(), |el, k| el.child(key_span(k)))
                .child(div().flex_shrink_0().text_color(c_secondary).child(open))
                .child(div().flex_shrink_0().text_color(dim).child(summary))
                .child(div().flex_shrink_0().text_color(c_secondary).child(close))
                .into_any_element()
        }
    }
}
