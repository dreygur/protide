use gpui::{Hsla, StyledText, TextRun, div, font, prelude::*, px};

/// Check if character is a word character (alphanumeric or underscore)
pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Find the start of a word at the given position
pub fn find_word_start(text: &str, pos: usize) -> usize {
    if text.is_empty() || pos == 0 {
        return 0;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut start = pos.min(chars.len().saturating_sub(1));

    // If on whitespace, move back to find a word
    while start > 0 && !is_word_char(chars[start]) {
        start -= 1;
    }

    // Find the start of this word
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    start
}

/// Find the end of a word at the given position
pub fn find_word_end(text: &str, pos: usize) -> usize {
    if text.is_empty() {
        return 0;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut end = pos.min(chars.len().saturating_sub(1));

    // If on whitespace, move forward to find a word
    while end < chars.len() && !is_word_char(chars[end]) {
        end += 1;
    }

    // Find the end of this word
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }
    end
}

/// Order a selection low-to-high and clamp it to the text.
///
/// `selection` is a **character** range (that is what `index_for_x`,
/// `find_word_*` and `char_to_byte_offset` all speak), so it must be clamped by
/// the character count. Clamping by `text.len()` - the *byte* length - lets a
/// stale selection over multi-byte text stay larger than the text really is,
/// and every caller below multiplies these numbers by a per-character width, so
/// the cursor and highlight get drawn past the end of the string.
fn normalized_selection(text: &str, selection: &std::ops::Range<usize>) -> (usize, usize) {
    let n = text.chars().count();
    (
        selection.start.min(selection.end).min(n),
        selection.start.max(selection.end).min(n),
    )
}

/// Render text with selection highlighting and cursor
///
/// Render text with optional max character limit for truncation
/// When focused, expands to multiple lines if chars_per_line is provided
/// `scroll_offset_x`: horizontal pixel scroll offset (0.0 = no scroll)
pub fn render_text_view_with_max(
    text: &str,
    selection: &std::ops::Range<usize>,
    is_focused: bool,
    font_size: f32,
    text_color: Hsla,
    placeholder: Option<&str>,
    placeholder_color: Hsla,
    max_chars: Option<usize>,
    selection_bg: Hsla,
) -> gpui::AnyElement {
    render_text_view_with_max_scrolled(
        text,
        selection,
        is_focused,
        font_size,
        text_color,
        placeholder,
        placeholder_color,
        max_chars,
        selection_bg,
        0.0,
    )
}

pub fn render_text_view_with_max_scrolled(
    text: &str,
    selection: &std::ops::Range<usize>,
    is_focused: bool,
    font_size: f32,
    text_color: Hsla,
    placeholder: Option<&str>,
    placeholder_color: Hsla,
    max_chars: Option<usize>,
    selection_bg: Hsla,
    scroll_offset_x: f32,
) -> gpui::AnyElement {
    // Use default chars_per_line based on max_chars for multi-line when focused
    let chars_per_line = max_chars.map(|m| m.max(10));
    render_text_view_multiline(
        text,
        selection,
        is_focused,
        font_size,
        text_color,
        placeholder,
        placeholder_color,
        max_chars,
        chars_per_line,
        selection_bg,
        scroll_offset_x,
    )
}

/// Render text with multi-line support when focused
/// - `max_chars`: truncation limit when unfocused
/// - `chars_per_line`: characters per line when focused (enables multi-line wrapping)
/// - `scroll_offset_x`: horizontal pixel offset for single-line scroll (0.0 = no scroll)
pub fn render_text_view_multiline(
    text: &str,
    selection: &std::ops::Range<usize>,
    is_focused: bool,
    font_size: f32,
    text_color: Hsla,
    placeholder: Option<&str>,
    placeholder_color: Hsla,
    max_chars: Option<usize>,
    chars_per_line: Option<usize>,
    selection_bg: Hsla,
    scroll_offset_x: f32,
) -> gpui::AnyElement {
    use gpui::IntoElement;

    if text.is_empty() {
        if let Some(ph) = placeholder
            && !is_focused
        {
            return div()
                .flex()
                .items_center()
                .text_size(px(font_size))
                .font_family("JetBrains Mono")
                .text_color(placeholder_color)
                .child(ph.to_string())
                .into_any_element();
        }
        // Empty but focused - show cursor
        return div()
            .flex()
            .items_center()
            .text_size(px(font_size))
            .font_family("JetBrains Mono")
            .text_color(text_color)
            .when(is_focused, |el| {
                el.child(div().w(px(1.0)).h(px(font_size + 2.0)).bg(text_color))
            })
            .into_any_element();
    }

    // Unfocused: show truncated text with ellipsis (single line)
    if !is_focused {
        let display_text = if let Some(max) = max_chars {
            if text.chars().count() > max {
                let truncated: String = text.chars().take(max.saturating_sub(1)).collect();
                format!("{}…", truncated)
            } else {
                text.to_string()
            }
        } else {
            text.to_string()
        };

        return div()
            .flex()
            .items_center()
            .text_size(px(font_size))
            .font_family("JetBrains Mono")
            .text_color(text_color)
            .child(display_text)
            .into_any_element();
    }

    // Focused: show full text with multi-line wrapping
    let (sel_start, sel_end) = normalized_selection(text, selection);
    let has_sel = sel_start != sel_end;
    let cursor_pos = sel_end;

    // Character width for fixed-width rendering
    let char_width = font_size * 0.6;

    // If no chars_per_line specified, render single line with fixed-width chars
    let cpl = match chars_per_line {
        Some(c) if c > 0 => c,
        _ => {
            // Single line with fixed-width character rendering
            return div()
                .flex()
                .items_center()
                .h_full()
                .relative()
                .left(px(-scroll_offset_x))
                // Selection highlight (absolute positioned)
                .when(has_sel, |el| {
                    let sel_x = sel_start as f32 * char_width;
                    let sel_width = (sel_end - sel_start) as f32 * char_width;
                    el.child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(px(sel_x))
                            .w(px(sel_width))
                            .bg(selection_bg),
                    )
                })
                // Cursor (absolute positioned, centered vertically)
                .child(
                    div()
                        .absolute()
                        .top_1()
                        .bottom_1()
                        .left(px(cursor_pos as f32 * char_width))
                        .w(px(2.0))
                        .bg(text_color),
                )
                .child(div().text_size(px(font_size)).child(
                    StyledText::new(text.to_string()).with_runs(vec![TextRun {
                        len: text.len(),
                        font: font("JetBrains Mono"),
                        color: text_color,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }]),
                ))
                .into_any_element();
        }
    };

    // Break text into lines
    let chars: Vec<char> = text.chars().collect();
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();

    for ch in &chars {
        current_line.push(*ch);
        if current_line.chars().count() >= cpl {
            lines.push(current_line);
            current_line = String::new();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }

    // Build multi-line display with fixed-width character rendering
    let line_height = font_size + 4.0;

    div()
        .flex()
        .flex_col()
        .w_full()
        .text_size(px(font_size))
        .font_family("JetBrains Mono")
        .text_color(text_color)
        .children(lines.iter().enumerate().map(|(line_idx, line_text)| {
            let line_start = line_idx * cpl;
            let line_end = line_start + line_text.chars().count();

            // Check if cursor/selection is on this line
            let cursor_on_line = !has_sel && cursor_pos >= line_start && cursor_pos <= line_end;
            let sel_intersects = has_sel && sel_start < line_end && sel_end > line_start;

            // Calculate local positions
            let local_sel_start = sel_start
                .saturating_sub(line_start)
                .min(line_text.chars().count());
            let local_sel_end = sel_end
                .saturating_sub(line_start)
                .min(line_text.chars().count());
            let local_cursor = cursor_pos
                .saturating_sub(line_start)
                .min(line_text.chars().count());

            div()
                .h(px(line_height))
                .flex()
                .items_center()
                .relative()
                // Selection highlight (absolute positioned)
                .when(sel_intersects, |el| {
                    let sel_x = local_sel_start as f32 * char_width;
                    let sel_width = (local_sel_end - local_sel_start) as f32 * char_width;
                    el.child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(px(sel_x))
                            .w(px(sel_width.max(2.0)))
                            .bg(selection_bg),
                    )
                })
                // Cursor (absolute positioned)
                .when(
                    cursor_on_line
                        || (sel_intersects && sel_end >= line_start && sel_end <= line_end),
                    |el| {
                        let cursor_x = if cursor_on_line {
                            local_cursor
                        } else {
                            local_sel_end
                        };
                        el.child(
                            div()
                                .absolute()
                                .top(px(2.0))
                                .left(px(cursor_x as f32 * char_width))
                                .w(px(2.0))
                                .h(px(font_size + 2.0))
                                .bg(text_color),
                        )
                    },
                )
                .child(div().text_size(px(font_size)).child(
                    StyledText::new(line_text.clone()).with_runs(vec![TextRun {
                        len: line_text.len(),
                        font: font("JetBrains Mono"),
                        color: text_color,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }]),
                ))
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_word_char ─────────────────────────────────────────────────────────

    #[test]
    fn word_characters_are_alphanumerics_and_underscore() {
        for c in ['a', 'Z', '0', '9', '_', 'é', 'ü', '日', 'Ω'] {
            assert!(is_word_char(c), "{c:?} should be a word char");
        }
        for c in [' ', '\t', '\n', '/', ':', '?', '=', '&', '.', '-', '🎉'] {
            assert!(!is_word_char(c), "{c:?} should not be a word char");
        }
    }

    // ── find_word_start / find_word_end ──────────────────────────────────────

    /// Build a range from runtime values - a literal `b..a` with a > b trips
    /// clippy::reversed_empty_ranges even where reversal is the point.
    fn rng(a: usize, b: usize) -> std::ops::Range<usize> {
        a..b
    }

    /// Word span selected by a double-click at char index `pos`.
    fn word_at(text: &str, pos: usize) -> String {
        let (s, e) = (find_word_start(text, pos), find_word_end(text, pos));
        assert!(s <= e, "inverted word span {s}..{e} for {text:?} @ {pos}");
        text.chars().skip(s).take(e - s).collect()
    }

    #[test]
    fn a_double_click_inside_a_word_selects_that_whole_word() {
        let text = "GET /api/users?id=42";
        assert_eq!(word_at(text, 0), "GET");
        assert_eq!(word_at(text, 2), "GET");
        assert_eq!(word_at(text, 5), "api");
        assert_eq!(word_at(text, 10), "users");
        assert_eq!(word_at(text, 18), "42");
    }

    #[test]
    fn word_boundaries_are_char_indices_not_byte_indices() {
        // Both functions index a Vec<char>, so their results must be counted in
        // characters; treating them as byte offsets would slice mid-character.
        let text = "日本語 hello";
        assert_eq!(find_word_start(text, 1), 0);
        assert_eq!(
            find_word_end(text, 1),
            3,
            "the CJK word is 3 chars, not 9 bytes"
        );
        assert_eq!(word_at(text, 1), "日本語");
        assert_eq!(word_at(text, 5), "hello");
    }

    // KNOWN DEFECT (not fixed): `find_word_start` and `find_word_end` scan in
    // *opposite* directions when the click lands on a separator.
    // `find_word_start` walks backwards to the previous word (text_view.rs:17)
    // while `find_word_end` walks forwards to the next one (text_view.rs:37), so
    // a double-click on any non-word character selects the preceding word, the
    // separators, *and* the following word instead of a single word. Trigger:
    // double-click a space or a URL separator - e.g. on "https://api.example.com"
    // at the ':' the selection becomes "https://api". The existing
    // `test_find_word_start_simple` / `test_find_word_end_simple` pair in
    // panels/request/tests.rs already pins both halves of this contradiction
    // (position 5 of "hello world" gives start 0 and end 11).
    //
    // Left unfixed deliberately: making the two agree is a user-visible change
    // to double-click semantics (select the following word? the preceding one?
    // the run of separators?), which is a product decision, not a mechanical fix.
    #[test]
    #[ignore = "known defect: find_word_start scans backwards while find_word_end scans forwards"]
    fn a_double_click_on_a_separator_selects_only_one_word() {
        assert_eq!(word_at("a  bb", 1), "bb");
        assert_eq!(word_at("hello world", 5), "world");
        assert_eq!(word_at("https://api.example.com", 5), "https");
    }

    #[test]
    fn word_lookup_never_indexes_past_the_end_of_the_text() {
        for text in ["", " ", "a", "word", "日本語", "e\u{0301}x", "🎉🎉"] {
            let n = text.chars().count();
            for pos in 0..n + 5 {
                let (s, e) = (find_word_start(text, pos), find_word_end(text, pos));
                assert!(s <= n, "{text:?} @ {pos}: start {s} > {n}");
                assert!(e <= n, "{text:?} @ {pos}: end {e} > {n}");
                assert!(s <= e, "{text:?} @ {pos}: inverted {s}..{e}");
            }
        }
    }

    #[test]
    fn word_lookup_on_empty_text_is_an_empty_span() {
        assert_eq!(find_word_start("", 0), 0);
        assert_eq!(find_word_start("", 99), 0);
        assert_eq!(find_word_end("", 0), 0);
        assert_eq!(find_word_end("", 99), 0);
    }

    #[test]
    fn text_with_no_word_characters_yields_an_empty_selection() {
        let text = "///";
        assert_eq!(find_word_end(text, 0), 3);
        assert_eq!(find_word_start(text, 2), 0);
    }

    // KNOWN DEFECT (not fixed): word selection is codepoint-based, so a
    // decomposed grapheme (base letter + combining mark) is split - the mark is
    // not alphanumeric, so `find_word_end` stops before it and a double-click on
    // "café" spelled as "cafe\u{0301}" copies "cafe", silently dropping the
    // accent. Precomposed "café" (U+00E9) is handled correctly. Fixing this
    // needs grapheme-cluster segmentation, which would mean a new dependency,
    // so the behaviour is documented here rather than changed.
    #[test]
    #[ignore = "known defect: codepoint-based word selection splits decomposed graphemes"]
    fn a_double_click_selects_a_whole_grapheme_cluster() {
        assert_eq!(word_at("cafe\u{0301} x", 1), "cafe\u{0301}");
    }

    // ── render_text_view_multiline: must survive any selection ───────────────

    fn render_all_shapes(text: &str, sel: std::ops::Range<usize>) {
        for is_focused in [true, false] {
            for max_chars in [None, Some(0), Some(1), Some(5), Some(1000)] {
                for chars_per_line in [None, Some(0), Some(1), Some(4), Some(1000)] {
                    let _ = render_text_view_multiline(
                        text,
                        &sel,
                        is_focused,
                        12.0,
                        gpui::black(),
                        Some("placeholder"),
                        gpui::white(),
                        max_chars,
                        chars_per_line,
                        gpui::white(),
                        0.0,
                    );
                }
            }
        }
    }

    #[test]
    fn rendering_never_panics_for_any_selection_over_multibyte_text() {
        // Selections are *character* ranges that can outrun the text after an
        // edit, so every combination has to be survivable.
        for text in ["", "ascii", "日本語", "e\u{0301}🎉x", "a b  c\td"] {
            let n = text.chars().count();
            for start in 0..n + 3 {
                for end in 0..n + 3 {
                    render_all_shapes(text, rng(start, end));
                }
            }
            render_all_shapes(text, rng(0, usize::MAX));
            render_all_shapes(text, rng(usize::MAX, 0));
        }
    }

    // FIXED: the focused-render path clamped its selection with
    // `.min(text.len())`. The selection is a *character* range but `len()` is a
    // *byte* count, so over multi-byte text the clamp was far too loose and a
    // stale selection drew the cursor / highlight past the end of the string
    // (every use of these numbers scales them by a per-character width).
    #[test]
    fn a_selection_is_clamped_by_character_count_not_byte_length() {
        // "日本語" is 3 characters but 9 bytes.
        assert_eq!(normalized_selection("日本語", &(0..9)), (0, 3));
        assert_eq!(normalized_selection("日本語", &rng(5, 7)), (3, 3));
        assert_eq!(normalized_selection("日本語", &(1..2)), (1, 2));
        assert_eq!(normalized_selection("", &(0..4)), (0, 0));
        assert_eq!(normalized_selection("ascii", &(0..99)), (0, 5));
    }

    #[test]
    fn a_backwards_selection_is_ordered_low_to_high() {
        assert_eq!(normalized_selection("hello", &rng(4, 1)), (1, 4));
        assert_eq!(normalized_selection("hello", &(1..4)), (1, 4));
        assert_eq!(normalized_selection("hello", &rng(usize::MAX, 0)), (0, 5));
    }

    #[test]
    fn a_normalized_selection_can_always_index_the_characters_it_spans() {
        for text in ["", "ascii", "日本語", "e\u{0301}🎉x"] {
            let n = text.chars().count();
            for a in 0..n + 3 {
                for b in 0..n + 3 {
                    let (s, e) = normalized_selection(text, &rng(a, b));
                    assert!(s <= e && e <= n, "{text:?} {a}..{b} -> {s}..{e}");
                    let _: String = text.chars().skip(s).take(e - s).collect();
                }
            }
        }
    }

    #[test]
    fn an_unfocused_empty_field_renders_its_placeholder() {
        // Smoke-only: element trees cannot be inspected here, so this just pins
        // that the empty/placeholder path is reachable without panicking.
        let _ = render_text_view_with_max(
            "",
            &(0..0),
            false,
            12.0,
            gpui::black(),
            Some("https://"),
            gpui::white(),
            Some(40),
            gpui::white(),
        );
    }

    #[test]
    fn truncation_of_multibyte_text_does_not_split_a_character() {
        // Exercises the `max_chars` ellipsis path, which slices by chars().take.
        for max in [0, 1, 2, 3, 10] {
            let _ = render_text_view_with_max(
                "日本語のテキスト",
                &(0..0),
                false,
                12.0,
                gpui::black(),
                None,
                gpui::white(),
                Some(max),
                gpui::white(),
            );
        }
    }
}
